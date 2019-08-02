use crate::libp2p_bam::{
    handler::{PendingInboundResponse, PendingOutboundRequest, ProtocolOutEvent},
    protocol::{BamProtocol, BamStream},
    substream::{Advance, Advanced, CloseStream},
};
use bam::json::{Frame, FrameType, Response};
use futures::sync::oneshot;
use libp2p::core::protocols_handler::{ProtocolsHandlerEvent, SubstreamProtocol};
use std::collections::{HashMap, HashSet};
use tokio::prelude::*;

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an outbound substream i.e. from us to peer node.
pub enum State<TSubstream> {
    WaitingOpen {
        req: PendingOutboundRequest,
    },
    WaitingSend {
        msg: Frame,
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    WaitingFlush {
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    WaitingAnswer {
        response_sender: oneshot::Sender<Response>,
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
                                ProtocolOutEvent::UnexpectedFrameType {
                                    bad_frame: frame,
                                    expected_type,
                                },
                            )),
                        };
                    }

                    let event = serde_json::from_value(frame.payload)
                        .map(|response| {
                            ProtocolOutEvent::InboundResponse(PendingInboundResponse {
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

                            ProtocolOutEvent::BadInboundResponse
                        });

                    Advanced {
                        new_state: Some(WaitingClose { stream }),
                        event: Some(ProtocolsHandlerEvent::Custom(event)),
                    }
                }
                Ok(Async::Ready(None)) => Advanced {
                    new_state: Some(State::WaitingClose { stream }),
                    event: Some(ProtocolsHandlerEvent::Custom(
                        ProtocolOutEvent::UnexpectedEOF,
                    )),
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
