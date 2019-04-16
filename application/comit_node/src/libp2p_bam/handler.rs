use crate::libp2p_bam::protocol::BamConfig;
use bam::{
    json::{
        Frame, Header, JsonFrameCodec, OutgoingRequest, Response, UnknownMandatoryHeaders,
        UnvalidatedIncomingRequest, ValidatedIncomingRequest,
    },
    IntoFrame, Status,
};
use derivative::Derivative;
use futures::{
    sink::Sink,
    stream::Stream,
    sync::oneshot,
    task::{self, Task},
    Async, AsyncSink, Future,
};
use libp2p::core::{
    protocols_handler::{
        KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr,
    },
    upgrade::Negotiated,
};
use std::{
    collections::{vec_deque::VecDeque, HashMap, HashSet},
    convert::Infallible,
    marker::PhantomData,
};
use tokio::{
    codec::Framed,
    prelude::{AsyncRead, AsyncWrite},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    marker: PhantomData<TSubstream>,
    #[derivative(Debug = "ignore")]
    framed: Option<Framed<Negotiated<TSubstream>, JsonFrameCodec>>,

    next_outgoing_id: u32,
    pending_outgoing_request_channels: HashMap<u32, oneshot::Sender<Response>>,

    next_incoming_id: u32,

    #[derivative(Debug = "ignore")]
    pending_incoming_request_channels:
        HashMap<u32, Box<dyn Future<Item = Response, Error = ()> + Send>>,

    pending_frames: VecDeque<Frame>,

    known_headers: HashMap<String, HashSet<String>>,

    current_task: Option<Task>,
    waiting_for_outbound_stream_task: Option<Task>,
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            marker: PhantomData,
            framed: None,
            next_outgoing_id: 0,
            pending_outgoing_request_channels: HashMap::new(),
            next_incoming_id: 0,
            pending_incoming_request_channels: HashMap::new(),
            pending_frames: VecDeque::new(),
            known_headers,
            current_task: None,
            waiting_for_outbound_stream_task: None,
        }
    }
}

#[derive(Debug)]
pub struct PendingOutgoingRequest {
    pub request: OutgoingRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingIncomingRequest {
    pub request: ValidatedIncomingRequest,
    pub channel: oneshot::Sender<Response>,
}

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = PendingOutgoingRequest;
    type OutEvent = PendingIncomingRequest;
    type Error = bam::json::Error;
    type Substream = TSubstream;
    type InboundProtocol = BamConfig;
    type OutboundProtocol = BamConfig;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> Self::InboundProtocol {
        BamConfig {}
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.framed = Some(protocol);

        log::debug!("received fully negotiated connection!");

        if let Some(task) = &self.current_task {
            task.notify();
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        _info: Self::OutboundOpenInfo,
    ) {
        self.framed = Some(protocol);

        log::debug!("received fully negotiated connection!");

        if let Some(task) = &self.waiting_for_outbound_stream_task {
            task.notify();
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        log::debug!("received event: {:?}", event.request);

        let PendingOutgoingRequest { request, channel } = event;

        let frame = request.into_frame(self.next_outgoing_id);
        self.pending_frames.push_back(frame);

        self.pending_outgoing_request_channels
            .insert(self.next_outgoing_id, channel);

        self.next_outgoing_id += 1;

        if let Some(task) = &self.current_task {
            task.notify();
        }
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        _error: ProtocolsHandlerUpgrErr<Infallible>,
    ) {
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Forever
    }

    fn poll(
        &mut self,
    ) -> Result<
        Async<
            ProtocolsHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::OutEvent>,
        >,
        Self::Error,
    > {
        log::debug!("polling handler.");
        log::debug!(
            "{} pending requests",
            self.pending_incoming_request_channels.len()
        );
        log::debug!("{} pending frames", self.pending_frames.len());

        for (id, future) in self.pending_incoming_request_channels.iter_mut() {
            match future.poll() {
                Ok(Async::Ready(response)) => {
                    let frame = response.into_frame(*id);

                    self.pending_frames.push_back(frame);

                    log::debug!("response for request {} is ready, created new frame", id)
                }
                Ok(Async::NotReady) => {
                    // FIXME: should we really early return here?
                    return Ok(Async::NotReady);
                }
                Err(_) => log::warn!("error while polling response for request {}", id),
            }
        }

        match self.framed.as_mut() {
            Some(framed) => {
                match framed.poll_complete() {
                    Ok(Async::Ready(_)) => log::debug!("sink successfully flushed!"),
                    Ok(Async::NotReady) => {
                        log::debug!("more items to be sent over the sink");
                        return Ok(Async::NotReady);
                    }
                    Err(e) => return Err(e),
                }

                if let Some(frame) = self.pending_frames.pop_front() {
                    match framed.start_send(frame) {
                        Ok(AsyncSink::Ready) => {
                            // cool, we sent it!

                            match framed.poll_complete() {
                                Ok(Async::Ready(_)) => log::debug!("sink successfully flushed!"),
                                Ok(Async::NotReady) => {
                                    log::debug!("more items to be sent over the sink");
                                    return Ok(Async::NotReady);
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        Ok(AsyncSink::NotReady(pending_frame)) => {
                            self.pending_frames.push_front(pending_frame);

                            log::debug!("sink is full, returning Async::NotReady");

                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e),
                    }
                }

                match framed.poll() {
                    Ok(Async::Ready(Some(frame))) => {
                        let id = frame.id;

                        let option = match frame.frame_type.as_str() {
                            "REQUEST" => self.handle_request(frame),
                            "RESPONSE" => {
                                self.handle_response(frame);
                                None
                            }
                            _ => {
                                self.handle_unknown_frame(frame);
                                None
                            }
                        };

                        match option {
                            Some(request) => {
                                log::debug!("got new incoming request, emitting event");
                                return Ok(Async::Ready(ProtocolsHandlerEvent::Custom(request)));
                            }
                            None => {
                                // TODO: this is probably wrong
                                self.current_task = Some(task::current());

                                log::debug!("handling frame {} yielded no element, returning Async::NotReady", id);

                                return Ok(Async::NotReady);
                            }
                        }
                    }
                    Ok(Async::Ready(None)) => unimplemented!("TBD"),
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e),
                }
            }
            None => {
                log::debug!("connection to remote not yet fully established, cannot do any work");

                match self.waiting_for_outbound_stream_task {
                    Some(_) => {
                        self.waiting_for_outbound_stream_task = Some(task::current());

                        return Ok(Async::NotReady);
                    }
                    None => {
                        self.waiting_for_outbound_stream_task = Some(task::current());

                        return Ok(Async::Ready(
                            ProtocolsHandlerEvent::OutboundSubstreamRequest {
                                upgrade: BamConfig {},
                                info: (),
                            },
                        ));
                    }
                }
            }
        }
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    fn handle_request(&mut self, frame: Frame) -> Option<PendingIncomingRequest> {
        if frame.id < self.next_incoming_id {
            log::warn!(
                "received out-of-order request with id {}. next expected id was {}",
                frame.id,
                self.next_incoming_id
            );
            return None;
        }

        self.next_incoming_id = frame.id + 1;

        let request = serde_json::from_value(frame.payload)
            .map_err(malformed_request)
            .and_then(|request| self.validate_request(request));

        match request {
            Ok(validated_incoming_request) => {
                let (sender, receiver) = oneshot::channel();

                let channel = Box::new(receiver.map_err(|_| {
                    log::warn!(
                        "Sender of response future was unexpectedly dropped before response was received."
                    )
                }));

                self.pending_incoming_request_channels
                    .insert(frame.id, channel);

                return Some(PendingIncomingRequest {
                    request: validated_incoming_request,
                    channel: sender,
                });
            }
            Err(error_response) => {
                let frame = error_response.into_frame(frame.id);
                self.pending_frames.push_back(frame);

                return None;
            }
        }
    }

    fn handle_response(&mut self, frame: Frame) {
        let response = serde_json::from_value(frame.payload);

        let maybe_request_channel = self.pending_outgoing_request_channels.remove(&frame.id);

        match maybe_request_channel {
            Some(sender) => match response {
                Ok(response) => {
                    log::debug!("dispatching {:?} to stored handler", response);
                    sender.send(response).unwrap()
                }
                Err(e) => log::error!(
                    "payload of frame {} is not a well-formed RESPONSE: {:?}",
                    frame.id,
                    e
                ),
            },
            None => log::warn!("received unexpected response with id {}", frame.id),
        }
    }

    fn handle_unknown_frame(&mut self, frame: Frame) {
        log::warn!("received frame with unknown type {}", frame.frame_type)
    }

    fn validate_request(
        &self,
        request: UnvalidatedIncomingRequest,
    ) -> Result<ValidatedIncomingRequest, Response> {
        self.known_headers
            .get(request.request_type())
            .ok_or_else(|| unknown_request_type(request.request_type()))
            .and_then(|known_headers| {
                request
                    .ensure_no_unknown_mandatory_headers(known_headers)
                    .map_err(unknown_mandatory_headers)
            })
    }
}

fn malformed_request(error: serde_json::Error) -> Response {
    log::warn!("incoming request was malformed: {:?}", error);

    Response::new(Status::SE(0))
}

fn unknown_request_type(request_type: &str) -> Response {
    log::warn!("request type '{}' is unknown", request_type);

    Response::new(Status::SE(2))
}

fn unknown_mandatory_headers(unknown_headers: UnknownMandatoryHeaders) -> Response {
    Response::new(Status::SE(1)).with_header(
        "Unsupported-Headers",
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}
