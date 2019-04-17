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
    mem,
};
use tokio::{
    codec::Framed,
    prelude::{AsyncRead, AsyncWrite},
};

enum SocketState<TSubstream> {
    Disconnected,
    Requested,
    Waiting(Task),
    Connected(Framed<Negotiated<TSubstream>, JsonFrameCodec>),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    known_headers: HashMap<String, HashSet<String>>,
    #[derivative(Debug = "ignore")]
    network_socket: SocketState<TSubstream>,
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
            network_socket: SocketState::Disconnected,
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
        let current_state =
            mem::replace(&mut self.network_socket, SocketState::Connected(protocol));

        if let SocketState::Waiting(task) = current_state {
            task.notify();
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        _info: Self::OutboundOpenInfo,
    ) {
        let current_state =
            mem::replace(&mut self.network_socket, SocketState::Connected(protocol));

        if let SocketState::Waiting(task) = current_state {
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

    fn poll(
        &mut self,
    ) -> Result<
        Async<
            ProtocolsHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::OutEvent>,
        >,
        Self::Error,
    > {
        self.poll_response_futures();

        match &mut self.network_socket {
            SocketState::Connected(socket) => {
                futures::try_ready!(socket.poll_complete());

                while let Some(frame) = self.pending_frames.pop_front() {
                    let message = if log::log_enabled!(target: "bam", log::Level::Trace) {
                        format!("--> OUTGOING FRAME {:?}", frame)
                    } else {
                        format!("")
                    };

                    match socket.start_send(frame) {
                        Ok(AsyncSink::Ready) => {
                            log::trace!(target: "bam", "{}", message);
                            futures::try_ready!(socket.poll_complete());
                        }
                        Ok(AsyncSink::NotReady(pending_frame)) => {
                            self.pending_frames.push_front(pending_frame);
                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e),
                    }
                }

                match futures::try_ready!(socket.poll()) {
                    Some(frame) => {
                        log::trace!(target: "bam", "<-- INCOMING FRAME {:?}", frame);
                        if let Some(request) = self.handle_frame(frame) {
                            return Ok(Async::Ready(ProtocolsHandlerEvent::Custom(request)));
                        }

                        return Ok(Async::NotReady);
                    }
                    None => {
                        log::info!(target: "bam", "substream is closed, trying to reconnect");
                        self.network_socket = SocketState::Requested;

                        return Ok(Async::Ready(
                            ProtocolsHandlerEvent::OutboundSubstreamRequest {
                                upgrade: BamConfig {},
                                info: (),
                            },
                        ));
                    }
                }
            }
            SocketState::Disconnected => {
                self.network_socket = SocketState::Requested;

                return Ok(Async::Ready(
                    ProtocolsHandlerEvent::OutboundSubstreamRequest {
                        upgrade: BamConfig {},
                        info: (),
                    },
                ));
            }
            SocketState::Requested | SocketState::Waiting(_) => {
                self.network_socket = SocketState::Waiting(task::current());

                return Ok(Async::NotReady);
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
                    self.pending_incoming_request_channels.remove(index)
                }
                Ok(Async::NotReady) => {
                    index += 1;
                }
                Err(e) => {
                    log::warn!(target: "bam", "polling response future for frame {} yielded an error", frame_id);
                    self.pending_incoming_request_channels.remove(index)
                }
            }
        }
    }

    fn handle_frame(&mut self, frame: Frame) -> Option<PendingIncomingRequest> {
        match frame.frame_type.as_str() {
            "REQUEST" => self.handle_request(frame),
            "RESPONSE" => {
                self.handle_response(frame);
                None
            }
            _ => {
                self.handle_unknown_frame(frame);
                None
            }
        }
    }

    fn handle_request(&mut self, frame: Frame) -> Option<PendingIncomingRequest> {
        if frame.id < self.next_incoming_id {
            log::warn!(
                target: "bam",
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

                self.pending_incoming_request_channels
                    .push((frame.id, receiver));

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
                Ok(response) => sender.send(response).unwrap(),
                Err(e) => log::error!(
                    target: "bam",
                    "payload of frame {} is not a well-formed RESPONSE: {:?}",
                    frame.id,
                    e
                ),
            },
            None => log::warn!(target: "bam","received unexpected response with id {}", frame.id),
        }
    }

    fn handle_unknown_frame(&mut self, frame: Frame) {
        log::warn!(target: "bam","received frame with unknown type {}", frame.frame_type)
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
