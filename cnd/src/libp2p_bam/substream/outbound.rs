use crate::libp2p_bam::{
    handler::{
        self, InboundMessage, OutboundMessage, PendingInboundResponse, PendingOutboundRequest,
        ProtocolOutEvent, ProtocolOutboundOpenInfo,
    },
    protocol::{BamProtocol, BamStream},
    substream::{Advance, Advanced, CloseStream},
};
use bam::{frame::Response, Frame, FrameType};
use futures::sync::oneshot;
use libp2p::core::protocols_handler::{ProtocolsHandlerEvent, SubstreamProtocol};
use std::collections::{HashMap, HashSet};
use tokio::prelude::*;

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an outbound substream i.e. from us to peer node.
pub enum State<TSubstream> {
    /// We haven't started opening the outgoing substream yet.
    WaitingOpen { request: PendingOutboundRequest },
    /// Waiting to send a message to the remote.
    WaitingSend {
        frame: Frame,
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    /// Waiting to flush the substream so that the data arrives at the remote.
    WaitingFlush {
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
    /// Waiting for the answer to our message.
    WaitingAnswer {
        response_sender: oneshot::Sender<Response>,
        stream: BamStream<TSubstream>,
    },
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
            WaitingOpen { request } => {
                Advanced::emit_event(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(BamProtocol {}),
                    info: ProtocolOutboundOpenInfo::Message(OutboundMessage::Request(request)),
                })
            }
            WaitingSend {
                frame,
                response_sender,
                mut stream,
            } => match stream.start_send(frame) {
                Ok(AsyncSink::Ready) => WaitingFlush {
                    response_sender,
                    stream,
                }
                .advance(known_headers),
                Ok(AsyncSink::NotReady(frame)) => Advanced::transition_to(WaitingSend {
                    frame,
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
                Ok(Async::Ready(Some(frame))) => match frame.frame_type {
                    FrameType::Response => {
                        let event = serde_json::from_value(frame.payload)
                            .map(|response| {
                                ProtocolOutEvent::Message(InboundMessage::Response(
                                    PendingInboundResponse {
                                        response,
                                        channel: response_sender,
                                    },
                                ))
                            })
                            .map_err(handler::Error::MalformedFrame)
                            .unwrap_or_else(ProtocolOutEvent::Error);

                        Advanced {
                            new_state: Some(WaitingClose { stream }),
                            event: Some(ProtocolsHandlerEvent::Custom(event)),
                        }
                    }
                    FrameType::Request => {
                        Advanced::error(stream, handler::Error::UnexpectedFrame(frame))
                    }
                    FrameType::Unknown => Advanced::error(stream, handler::Error::UnknownFrameType),
                },
                Ok(Async::NotReady) => Advanced::transition_to(WaitingAnswer {
                    response_sender,
                    stream,
                }),
                Ok(Async::Ready(None)) => Advanced::error(stream, handler::Error::UnexpectedEOF),
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
