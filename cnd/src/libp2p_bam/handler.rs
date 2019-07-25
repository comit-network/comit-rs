use crate::libp2p_bam::{protocol::BamProtocol, BamStream, BehaviourInEvent};
use bam::{
    json::{
        Frame, FrameType, Header, JsonFrameCodec, OutgoingRequest, Response,
        UnknownMandatoryHeaders, UnvalidatedIncomingRequest, ValidatedIncomingRequest,
    },
    IntoFrame, Status,
};
use derivative::Derivative;
use futures::{
    sink::Sink,
    stream::Stream,
    sync::oneshot::{self, Canceled},
    task::Task,
    Async, AsyncSink, Future, Poll,
};
use libp2p::core::{
    protocols_handler::{
        KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr,
        SubstreamProtocol,
    },
    upgrade::Negotiated,
};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Display,
};
use tokio::{
    codec::Framed,
    prelude::{AsyncRead, AsyncWrite},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BamHandler<TSubstream> {
    #[derivative(Debug = "ignore")]
    incoming_substreams: Vec<InSubstreamState<TSubstream>>,
    #[derivative(Debug = "ignore")]
    outgoing_substreams: Vec<OutSubstreamState<TSubstream>>,

    #[derivative(Debug = "ignore")]
    current_task: Option<Task>,

    known_headers: HashMap<String, HashSet<String>>,
}

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// State of an active substream opened by us.
enum OutSubstreamState<TSubstream> {
    /// We haven't started opening the outgoing substream yet.
    WaitingOpen { req: PendingOutgoingRequest },
    /// Waiting to send a message to the remote.
    WaitingSend {
        msg: Frame,
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    /// Waiting to flush the substream so that the data arrives to the remote.
    WaitingFlush {
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    /// Waiting for the answer to our message
    WaitingAnswer {
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    /// The substream is being closed.
    WaitingClose { stream: BamStream<TSubstream> },
}

impl<TSubstream> CloseStream for OutSubstreamState<TSubstream> {
    type TSubstream = TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self {
        OutSubstreamState::WaitingClose { stream }
    }
}

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// State of an active substream opened by peer node.
enum InSubstreamState<TSubstream> {
    /// Waiting for a request from the remote.
    WaitingMessage { stream: BamStream<TSubstream> },
    /// Waiting for the user to send the response back to us.
    WaitingUser {
        response_receiver: oneshot::Receiver<Response>,
        stream: BamStream<TSubstream>,
    },
    /// Waiting to send an answer back to the remote.
    WaitingSend {
        msg: Frame,
        stream: BamStream<TSubstream>,
    },
    /// Waiting to flush an answer back to the remote.
    WaitingFlush { stream: BamStream<TSubstream> },
    /// The substream is being closed.
    WaitingClose { stream: BamStream<TSubstream> },
}

impl<TSubstream> CloseStream for InSubstreamState<TSubstream> {
    type TSubstream = TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self {
        InSubstreamState::WaitingClose { stream }
    }
}

#[allow(missing_debug_implementations)]
struct Advanced<S> {
    /// The optional new state we transitioned to.
    new_state: Option<S>,
    /// The optional event we generated as part of the transition.
    event: Option<BamHandlerEvent>,
}

trait Advance: Sized {
    fn advance(self, known_headers: &HashMap<String, HashSet<String>>) -> Advanced<Self>;
}

#[derive(Debug)]
pub enum Error {
    Stream(bam::json::Error),
    DroppedResponseSender(Canceled),
}

impl From<Canceled> for Error {
    fn from(e: Canceled) -> Self {
        Error::DroppedResponseSender(e)
    }
}

impl From<bam::json::Error> for Error {
    fn from(e: bam::json::Error) -> Self {
        Error::Stream(e)
    }
}

impl<S> Advanced<S> {
    fn transition_to(new_state: S) -> Self {
        Self {
            new_state: Some(new_state),
            event: None,
        }
    }

    fn emit_event(event: BamHandlerEvent) -> Self {
        Self {
            new_state: None,
            event: Some(event),
        }
    }

    fn end() -> Self {
        Self {
            new_state: None,
            event: None,
        }
    }
}

impl<S: CloseStream> Advanced<S> {
    fn error<E: Into<Error>>(stream: BamStream<S::TSubstream>, error: E) -> Self {
        let error = error.into();

        Self {
            new_state: Some(S::close(stream)),
            event: Some(ProtocolsHandlerEvent::Custom(InnerEvent::Error { error })),
        }
    }
}

trait CloseStream: Sized {
    type TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self;
}

impl<TSubstream: AsyncRead + AsyncWrite> Advance for OutSubstreamState<TSubstream> {
    fn advance(
        self,
        known_headers: &HashMap<String, HashSet<String>>,
    ) -> Advanced<OutSubstreamState<TSubstream>> {
        use self::OutSubstreamState::*;
        match self {
            WaitingOpen { req } => {
                Advanced::emit_event(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(BamProtocol {}),
                    info: req,
                })
            }
            WaitingSend {
                msg,
                response_sender,
                mut stream,
            } => match stream.start_send(msg) {
                Ok(AsyncSink::Ready) => WaitingFlush {
                    response_sender,
                    stream,
                }
                .advance(known_headers),
                Ok(AsyncSink::NotReady(msg)) => Advanced::transition_to(WaitingSend {
                    msg,
                    response_sender,
                    stream,
                }),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingFlush {
                response_sender,
                mut stream,
            } => match stream.poll_complete() {
                Ok(Async::Ready(_)) => Advanced::transition_to(WaitingAnswer {
                    response_sender,
                    stream,
                }),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingFlush {
                    response_sender,
                    stream,
                }),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingAnswer {
                response_sender,
                mut stream,
            } => match stream.poll() {
                Ok(Async::Ready(Some(frame))) => {
                    let expected_type = FrameType::Response;
                    if frame.frame_type != expected_type {
                        return Advanced {
                            new_state: Some(WaitingClose { stream }),
                            event: Some(ProtocolsHandlerEvent::Custom(
                                InnerEvent::UnexpectedFrameType {
                                    bad_frame: frame,
                                    expected_type,
                                },
                            )),
                        };
                    }

                    let event = serde_json::from_value(frame.payload)
                        .map(|response| {
                            InnerEvent::IncomingResponse(PendingIncomingResponse {
                                response,
                                channel: response_sender,
                            })
                        })
                        .unwrap_or_else(|deser_error| {
                            log::error!(
                                target: "bam",
                                "payload of frame is not a well-formed RESPONSE: {:?}",
                                deser_error
                            );

                            InnerEvent::BadIncomingResponse
                        });

                    Advanced {
                        new_state: Some(WaitingClose { stream }),
                        event: Some(ProtocolsHandlerEvent::Custom(event)),
                    }
                }
                Ok(Async::Ready(None)) => Advanced {
                    new_state: Some(OutSubstreamState::WaitingClose { stream }),
                    event: Some(ProtocolsHandlerEvent::Custom(InnerEvent::UnexpectedEOF)),
                },
                Ok(Async::NotReady) => Advanced::transition_to(WaitingAnswer {
                    response_sender,
                    stream,
                }),
                Err(error) => Advanced::error(stream, error),
            },

            WaitingClose { mut stream } => match stream.close() {
                Ok(Async::Ready(_)) => Advanced::end(),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingClose { stream }),
                Err(error) => Advanced::error(stream, error),
            },
        }
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> Advance for InSubstreamState<TSubstream> {
    fn advance(
        self,
        known_headers: &HashMap<String, HashSet<String>>,
    ) -> Advanced<InSubstreamState<TSubstream>> {
        use self::InSubstreamState::*;
        match self {
            WaitingMessage { mut stream } => match stream.poll() {
                Ok(Async::Ready(Some(frame))) => {
                    let expected_type = FrameType::Request;
                    if frame.frame_type != expected_type {
                        return Advanced {
                            new_state: Some(WaitingClose { stream }),
                            event: Some(ProtocolsHandlerEvent::Custom(
                                InnerEvent::UnexpectedFrameType {
                                    bad_frame: frame,
                                    expected_type,
                                },
                            )),
                        };
                    }

                    let request =
                        serde_json::from_value::<UnvalidatedIncomingRequest>(frame.payload)
                            .map_err(malformed_request_response)
                            .and_then(|request| {
                                known_headers
                                    .get(request.request_type())
                                    .ok_or_else(|| {
                                        unknown_request_type_response(request.request_type())
                                    })
                                    .and_then(|known_headers| {
                                        request
                                            .ensure_no_unknown_mandatory_headers(known_headers)
                                            .map_err(unknown_mandatory_headers_response)
                                    })
                            });

                    let (sender, receiver) = oneshot::channel();

                    Advanced {
                        new_state: Some(WaitingUser {
                            response_receiver: receiver,
                            stream,
                        }),
                        event: Some(ProtocolsHandlerEvent::Custom(match request {
                            Ok(request) => InnerEvent::IncomingRequest(PendingIncomingRequest {
                                request,
                                channel: sender,
                            }),
                            Err(response) => InnerEvent::BadIncomingRequest(
                                AutomaticallyGeneratedErrorResponse {
                                    response,
                                    channel: sender,
                                },
                            ),
                        })),
                    }
                }
                Ok(Async::Ready(None)) => Advanced {
                    new_state: Some(InSubstreamState::WaitingClose { stream }),
                    event: Some(ProtocolsHandlerEvent::Custom(InnerEvent::UnexpectedEOF)),
                },
                Ok(Async::NotReady) => Advanced::transition_to(WaitingMessage { stream }),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingUser {
                mut response_receiver,
                stream,
            } => match response_receiver.poll() {
                Ok(Async::Ready(response)) => WaitingSend {
                    msg: response.into_frame(),
                    stream,
                }
                .advance(known_headers),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingUser {
                    response_receiver,
                    stream,
                }),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingSend { msg, mut stream } => match stream.start_send(msg) {
                Ok(AsyncSink::Ready) => WaitingFlush { stream }.advance(known_headers),
                Ok(AsyncSink::NotReady(msg)) => {
                    Advanced::transition_to(WaitingSend { msg, stream })
                }
                Err(error) => Advanced::error(stream, error),
            },
            WaitingFlush { mut stream } => match stream.poll_complete() {
                Ok(Async::Ready(_)) => Advanced::transition_to(WaitingClose { stream }),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingFlush { stream }), //
                Err(error) => Advanced::error(stream, error),
            },

            WaitingClose { mut stream } => match stream.close() {
                Ok(Async::Ready(_)) => Advanced::end(),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingClose { stream }),
                Err(error) => Advanced::error(stream, error),
            },
        }
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            incoming_substreams: Vec::new(),
            outgoing_substreams: Vec::new(),
            current_task: None,
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
    BadIncomingResponse,
    UnexpectedFrameType {
        bad_frame: Frame,
        expected_type: FrameType,
    },
    UnexpectedEOF,
    Error {
        error: Error,
    },
}

type BamHandlerEvent = ProtocolsHandlerEvent<BamProtocol, PendingOutgoingRequest, InnerEvent>;

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = BehaviourInEvent;
    type OutEvent = InnerEvent;
    type Error = bam::json::Error;
    type Substream = TSubstream;
    type InboundProtocol = BamProtocol;
    type OutboundProtocol = BamProtocol;
    type OutboundOpenInfo = PendingOutgoingRequest;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(BamProtocol {})
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.incoming_substreams
            .push(InSubstreamState::WaitingMessage { stream });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        pending_incoming_request: Self::OutboundOpenInfo,
    ) {
        let PendingOutgoingRequest { request, channel } = pending_incoming_request;

        self.outgoing_substreams
            .push(OutSubstreamState::WaitingSend {
                msg: request.into_frame(),
                response_sender: channel,
                stream,
            });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            BehaviourInEvent::PendingOutgoingRequest { request } => {
                self.outgoing_substreams
                    .push(OutSubstreamState::WaitingOpen { req: request });

                if let Some(task) = &self.current_task {
                    task.notify()
                }
            }
        }
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        _error: ProtocolsHandlerUpgrErr<Infallible>,
    ) {
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(&mut self) -> Poll<BamHandlerEvent, Self::Error> {
        if let Some(result) = poll_substreams(&mut self.outgoing_substreams, &self.known_headers) {
            return result;
        }

        if let Some(result) = poll_substreams(&mut self.incoming_substreams, &self.known_headers) {
            return result;
        }

        self.current_task = Some(futures::task::current());

        Ok(Async::NotReady)
    }
}

fn poll_substreams<S: Display + Advance>(
    substreams: &mut Vec<S>,
    known_headers: &HashMap<String, HashSet<String>>,
) -> Option<Poll<BamHandlerEvent, bam::json::Error>> {
    log::debug!("polling {} substreams", substreams.len());

    // We remove each element from `substreams` one by one and add them back.
    for n in (0..substreams.len()).rev() {
        let substream_state = substreams.swap_remove(n);

        let log_message = format!("transition from {}", substream_state);

        let Advanced { new_state, event } = substream_state.advance(known_headers);

        if let Some(new_state) = new_state {
            log::trace!(target: "sub-libp2p", "{} to {}", log_message, new_state);
            substreams.push(new_state);
        }

        if let Some(event) = event {
            log::trace!(target: "sub-libp2p", "emitting {:?}", event);
            return Some(Ok(Async::Ready(event)));
        }
    }
    None
}

fn malformed_request_response(error: serde_json::Error) -> Response {
    log::warn!(target: "sub-libp2p", "incoming request was malformed: {:?}", error);

    Response::new(Status::SE(0))
}

fn unknown_request_type_response(request_type: &str) -> Response {
    log::warn!(target: "sub-libp2p", "request type '{}' is unknown", request_type);

    Response::new(Status::SE(2))
}

fn unknown_mandatory_headers_response(unknown_headers: UnknownMandatoryHeaders) -> Response {
    Response::new(Status::SE(1)).with_header(
        "Unsupported-Headers",
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}
