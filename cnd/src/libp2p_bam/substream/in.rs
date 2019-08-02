use crate::libp2p_bam::{
    handler::{AutomaticallyGeneratedErrorResponse, PendingIncomingRequest, ProtocolOutEvent},
    protocol::BamStream,
    substream::{Advance, Advanced, CloseStream},
};
use bam::{
    json::{
        Frame, FrameType, Header, Response, UnknownMandatoryHeaders, UnvalidatedIncomingRequest,
    },
    IntoFrame, Status,
};
use futures::sync::oneshot;
use libp2p::core::protocols_handler::ProtocolsHandlerEvent;
use std::collections::{HashMap, HashSet};
use tokio::prelude::*;

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an incoming substream i.e. from peer node to us.
pub enum State<TSubstream> {
    WaitingMessage {
        stream: BamStream<TSubstream>,
    },
    WaitingUser {
        response_receiver: oneshot::Receiver<Response>,
        stream: BamStream<TSubstream>,
    },
    WaitingSend {
        msg: Frame,
        stream: BamStream<TSubstream>,
    },
    WaitingFlush {
        stream: BamStream<TSubstream>,
    },
    WaitingClose {
        stream: BamStream<TSubstream>,
    },
}

impl<TSubstream> CloseStream for State<TSubstream> {
    type TSubstream = TSubstream;

    fn close(stream: BamStream<Self::TSubstream>) -> Self {
        State::WaitingClose { stream }
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> Advance for State<TSubstream> {
    fn advance(
        self,
        known_headers: &HashMap<String, HashSet<String>>,
    ) -> Advanced<State<TSubstream>> {
        use self::State::*;
        match self {
            WaitingMessage { mut stream } => match stream.poll() {
                Ok(Async::Ready(Some(frame))) => {
                    let expected_type = FrameType::Request;
                    if frame.frame_type != expected_type {
                        return Advanced {
                            new_state: Some(WaitingClose { stream }),
                            event: Some(ProtocolsHandlerEvent::Custom(
                                ProtocolOutEvent::UnexpectedFrameType {
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
                            Ok(request) => {
                                ProtocolOutEvent::IncomingRequest(PendingIncomingRequest {
                                    request,
                                    channel: sender,
                                })
                            }
                            Err(response) => ProtocolOutEvent::BadIncomingRequest(
                                AutomaticallyGeneratedErrorResponse {
                                    response,
                                    channel: sender,
                                },
                            ),
                        })),
                    }
                }
                Ok(Async::Ready(None)) => Advanced {
                    new_state: Some(State::WaitingClose { stream }),
                    event: Some(ProtocolsHandlerEvent::Custom(
                        ProtocolOutEvent::UnexpectedEOF,
                    )),
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
