use crate::libp2p_bam::{protocol::BamConfig, BamStream};
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
    Async, AsyncSink, Future, Poll,
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
    mem,
};
use tokio::{
    codec::Framed,
    prelude::{AsyncRead, AsyncWrite},
};

enum SubstreamState<TSubstream> {
    Disconnected,
    SubstreamRequested,
    WaitingForSubstream(Task),
    Connected(BamStream<TSubstream>),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    known_headers: HashMap<String, HashSet<String>>,
    #[derivative(Debug = "ignore")]
    network_stream: SubstreamState<TSubstream>,
    pending_frames: VecDeque<Frame>,

    next_outgoing_id: u32,
    pending_outgoing_request_channels: HashMap<u32, oneshot::Sender<Response>>,

    next_incoming_id: u32,
    pending_incoming_request_channels: Vec<(u32, oneshot::Receiver<Response>)>,
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            pending_frames: VecDeque::new(),
            network_stream: SubstreamState::Disconnected,
            next_outgoing_id: 0,
            pending_outgoing_request_channels: HashMap::new(),
            next_incoming_id: 0,
            pending_incoming_request_channels: Vec::new(),
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

#[derive(Debug)]
pub struct PendingIncomingResponse {
    pub response: Response,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct AutomaticallyGeneratedErrorResponse {
    pub response: Response,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub enum InnerEvent {
    IncomingRequest(PendingIncomingRequest),
    IncomingResponse(PendingIncomingResponse),
    BadIncomingRequest(AutomaticallyGeneratedErrorResponse),

    /// Error variant
    ///
    /// This currently covers all errors generated while handling incoming
    /// frames. Could potentially be expanded to pass more information to
    /// the NetworkBehaviour.
    Error,
}

type BamHandlerEvent = ProtocolsHandlerEvent<BamConfig, (), InnerEvent>;

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = PendingOutgoingRequest;
    type OutEvent = InnerEvent;
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
        let current_state = mem::replace(
            &mut self.network_stream,
            SubstreamState::Connected(protocol),
        );

        if let SubstreamState::WaitingForSubstream(task) = current_state {
            task.notify();
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        _info: Self::OutboundOpenInfo,
    ) {
        let current_state = mem::replace(
            &mut self.network_stream,
            SubstreamState::Connected(protocol),
        );

        if let SubstreamState::WaitingForSubstream(task) = current_state {
            task.notify();
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        let PendingOutgoingRequest { request, channel } = event;

        let frame = request.into_frame(self.next_outgoing_id);
        self.pending_frames.push_back(frame);

        self.pending_outgoing_request_channels
            .insert(self.next_outgoing_id, channel);

        self.next_outgoing_id += 1;
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

    fn poll(&mut self) -> Poll<BamHandlerEvent, Self::Error> {
        self.poll_response_futures();

        match &mut self.network_stream {
            SubstreamState::Connected(bam_stream) => {
                futures::try_ready!(bam_stream.poll_complete());

                while let Some(frame) = self.pending_frames.pop_front() {
                    let message = build_outgoing_log_message(&frame);

                    match bam_stream.start_send(frame) {
                        Ok(AsyncSink::Ready) => {
                            log::trace!(target: "bam", "{}", message);
                            futures::try_ready!(bam_stream.poll_complete());
                        }
                        Ok(AsyncSink::NotReady(pending_frame)) => {
                            self.pending_frames.push_front(pending_frame);
                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e),
                    }
                }

                match futures::try_ready!(bam_stream.poll()) {
                    Some(frame) => {
                        log::trace!(target: "bam", "<-- INCOMING FRAME {:?}", frame);

                        let inner_event = self.handle_frame(frame);
                        Ok(Async::Ready(ProtocolsHandlerEvent::Custom(inner_event)))
                    }
                    None => {
                        log::info!(target: "bam", "substream is closed, trying to reconnect");
                        self.network_stream = SubstreamState::SubstreamRequested;

                        Ok(Async::Ready(
                            ProtocolsHandlerEvent::OutboundSubstreamRequest {
                                upgrade: BamConfig {},
                                info: (),
                            },
                        ))
                    }
                }
            }
            SubstreamState::Disconnected => {
                self.network_stream = SubstreamState::SubstreamRequested;

                Ok(Async::Ready(
                    ProtocolsHandlerEvent::OutboundSubstreamRequest {
                        upgrade: BamConfig {},
                        info: (),
                    },
                ))
            }
            SubstreamState::SubstreamRequested | SubstreamState::WaitingForSubstream(_) => {
                self.network_stream = SubstreamState::WaitingForSubstream(task::current());

                Ok(Async::NotReady)
            }
        }
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    fn poll_response_futures(&mut self) {
        let mut index = 0;
        while index != self.pending_incoming_request_channels.len() {
            let (frame_id, response_future) = &mut self.pending_incoming_request_channels[index];

            match response_future.poll() {
                Ok(Async::Ready(response)) => {
                    let frame = response.into_frame(*frame_id);
                    self.pending_frames.push_back(frame);
                    self.pending_incoming_request_channels.remove(index);
                }
                Ok(Async::NotReady) => {
                    index += 1;
                }
                Err(_) => {
                    log::warn!(target: "bam", "polling response future for frame {} yielded an error", frame_id);
                    self.pending_incoming_request_channels.remove(index);
                }
            }
        }
    }

    fn handle_frame(&mut self, frame: Frame) -> InnerEvent {
        if frame.id < self.next_incoming_id {
            log::warn!(
                target: "bam",
                "received out-of-order request with id {}. next expected id was {}",
                frame.id,
                self.next_incoming_id
            );
            return InnerEvent::Error;
        }

        self.next_incoming_id = frame.id + 1;

        match frame.frame_type.as_str() {
            "REQUEST" => self.handle_request(frame),
            "RESPONSE" => self.handle_response(frame),
            _ => self.handle_unknown_frame(frame),
        }
    }

    fn handle_request(&mut self, frame: Frame) -> InnerEvent {
        let request = serde_json::from_value(frame.payload)
            .map_err(malformed_request)
            .and_then(|request| self.validate_request(request));

        let (sender, receiver) = oneshot::channel();

        self.pending_incoming_request_channels
            .push((frame.id, receiver));

        match request {
            Ok(request) => InnerEvent::IncomingRequest(PendingIncomingRequest {
                request,
                channel: sender,
            }),
            Err(response) => InnerEvent::BadIncomingRequest(AutomaticallyGeneratedErrorResponse {
                response,
                channel: sender,
            }),
        }
    }

    fn handle_response(&mut self, frame: Frame) -> InnerEvent {
        let response = serde_json::from_value(frame.payload);

        let maybe_request_channel = self.pending_outgoing_request_channels.remove(&frame.id);

        match maybe_request_channel {
            Some(sender) => match response {
                Ok(response) => InnerEvent::IncomingResponse(PendingIncomingResponse {
                    response,
                    channel: sender,
                }),
                Err(e) => {
                    log::error!(
                        target: "bam",
                        "payload of frame {} is not a well-formed RESPONSE: {:?}",
                        frame.id,
                        e
                    );
                    InnerEvent::Error
                }
            },
            None => {
                log::warn!(target: "bam","received unexpected response with id {}", frame.id);
                InnerEvent::Error
            }
        }
    }

    fn handle_unknown_frame(&mut self, frame: Frame) -> InnerEvent {
        log::warn!(target: "bam","received frame with unknown type {}", frame.frame_type);

        InnerEvent::Error
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
    log::warn!(target: "bam", "incoming request was malformed: {:?}", error);

    Response::new(Status::SE(0))
}

fn unknown_request_type(request_type: &str) -> Response {
    log::warn!(target: "bam", "request type '{}' is unknown", request_type);

    Response::new(Status::SE(2))
}

fn unknown_mandatory_headers(unknown_headers: UnknownMandatoryHeaders) -> Response {
    Response::new(Status::SE(1)).with_header(
        "Unsupported-Headers",
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}

fn build_outgoing_log_message(frame: &Frame) -> String {
    // This function only exists because we would have to clone every frame
    // otherwise since the frame is gone once we successfully sent it.
    // Hence we need to construct the message before we sent the frame.
    // To avoid creating a message if the log level is disabled, we check for that
    // before.

    if log::log_enabled!(target: "bam", log::Level::Trace) {
        format!("--> OUTGOING FRAME {:?}", frame)
    } else {
        String::new()
    }
}
