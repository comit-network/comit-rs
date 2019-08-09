use crate::libp2p_bam::{
    handler::{PendingInboundRequest, ProtocolOutEvent},
    protocol::BamStream,
    substream::{Advance, Advanced, CloseStream},
};
use bam::{
    frame::{ErrorType, Header, Response, UnknownMandatoryHeaders, UnvalidatedInboundRequest},
    Frame, FrameType, IntoFrame,
};
use futures::sync::oneshot;
use libp2p::core::protocols_handler::ProtocolsHandlerEvent;
use std::collections::{HashMap, HashSet};
use tokio::prelude::*;

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an inbound substream i.e. from peer node to us.
pub enum State<TSubstream> {
    /// Waiting for a request from the remote.
    WaitingMessage { stream: BamStream<TSubstream> },
    /// Waiting for the user to send the response back to us.
    WaitingUser {
        receiver: oneshot::Receiver<Response>,
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
                        serde_json::from_value::<UnvalidatedInboundRequest>(frame.payload)
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

                    match request {
                        Ok(request) => {
                            let (sender, receiver) = oneshot::channel();
                            return Advanced {
                                new_state: Some(WaitingUser { receiver, stream }),
                                event: Some(ProtocolsHandlerEvent::Custom(
                                    ProtocolOutEvent::InboundRequest(PendingInboundRequest {
                                        request,
                                        channel: sender,
                                    }),
                                )),
                            };
                        }
                        Err(response) => {
                            return Advanced::transition_to(WaitingSend {
                                msg: response.into_frame(),
                                stream,
                            });
                        }
                    };
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
                mut receiver,
                stream,
            } => match receiver.poll() {
                Ok(Async::Ready(response)) => WaitingSend {
                    msg: response.into_frame(),
                    stream,
                }
                .advance(known_headers),
                Ok(Async::NotReady) => Advanced::transition_to(WaitingUser { receiver, stream }),
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

// TODO: Revisit to check if error frame type assigned is correct
fn malformed_request_response(error: serde_json::Error) -> bam::frame::Error {
    log::warn!(target: "sub-libp2p", "incoming request was malformed: {:?}", error);

    bam::frame::Error::new(ErrorType::MalformedFrame)
}

fn unknown_request_type_response(request_type: &str) -> bam::frame::Error {
    log::warn!(target: "sub-libp2p", "request type '{}' is unknown", request_type);

    bam::frame::Error::new(ErrorType::UnknownRequestType)
}

fn unknown_mandatory_headers_response(
    unknown_headers: UnknownMandatoryHeaders,
) -> bam::frame::Error {
    bam::frame::Error::new(ErrorType::UnknownMandatoryHeader).with_details(
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}
