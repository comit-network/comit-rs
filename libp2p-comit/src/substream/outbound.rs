use crate::{
    frame::Response,
    handler::{self, InboundMessage, PendingInboundResponse, ProtocolOutEvent},
    protocol::Frames,
    substream::{Advance, Advanced, CloseStream},
    Frame, FrameKind,
};
use futures::{channel::oneshot, Sink, Stream};
use libp2p::swarm::ProtocolsHandlerEvent;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    task::{Context, Poll},
};

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an outbound substream i.e. from us to peer node.
pub enum State {
    /// Waiting to send a message to the remote.
    WaitingSend {
        frame: Frame,
        response_sender: oneshot::Sender<Response>,
        stream: Pin<Box<Frames>>,
    },
    /// Waiting to flush the substream so that the data arrives at the remote.
    WaitingFlush {
        response_sender: oneshot::Sender<Response>,
        stream: Pin<Box<Frames>>,
    },
    /// Waiting for the answer to our message.
    WaitingAnswer {
        response_sender: oneshot::Sender<Response>,
        stream: Pin<Box<Frames>>,
    },
    /// The substream is being closed.
    WaitingClose { stream: Pin<Box<Frames>> },
}

impl CloseStream for State {
    fn close(stream: Pin<Box<Frames>>) -> Self {
        State::WaitingClose { stream }
    }
}

impl Advance for State {
    fn advance(
        self,
        known_headers: &HashMap<String, HashSet<String>>,
        cx: &mut Context<'_>,
    ) -> Advanced<State> {
        use self::State::*;

        match self {
            WaitingSend {
                frame,
                response_sender,
                mut stream,
            } => match stream.as_mut().start_send(frame) {
                Ok(()) => WaitingFlush {
                    response_sender,
                    stream,
                }
                .advance(known_headers, cx),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingFlush {
                response_sender,
                mut stream,
            } => match stream.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => WaitingAnswer {
                    response_sender,
                    stream,
                }
                .advance(&known_headers, cx),
                Poll::Pending => Advanced::transition_to(WaitingFlush {
                    response_sender,
                    stream,
                }),
                Poll::Ready(Err(error)) => Advanced::error(stream, error),
            },
            WaitingAnswer {
                response_sender,
                mut stream,
            } => match stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(frame))) => {
                    let response = match frame.kind {
                        FrameKind::Response => serde_json::from_value(frame.payload),
                        FrameKind::Request => {
                            return Advanced::error(stream, handler::Error::UnexpectedFrame(frame))
                        }
                        FrameKind::Unknown => {
                            return Advanced::error(stream, handler::Error::UnknownFrameKind)
                        }
                    };

                    match response {
                        Ok(response) => {
                            let event = ProtocolOutEvent::Message(InboundMessage::Response(
                                PendingInboundResponse {
                                    response,
                                    channel: response_sender,
                                },
                            ));

                            Advanced {
                                new_state: Some(WaitingClose { stream }),
                                event: Some(ProtocolsHandlerEvent::Custom(event)),
                            }
                        }
                        Err(error) => {
                            Advanced::error(stream, handler::Error::MalformedFrame(error))
                        }
                    }
                }
                Poll::Ready(Some(Err(error))) => {
                    Advanced::error(stream, handler::Error::MalformedJson(error))
                }
                Poll::Pending => Advanced::transition_to(WaitingAnswer {
                    response_sender,
                    stream,
                }),
                Poll::Ready(None) => Advanced::error(stream, handler::Error::UnexpectedEOF),
            },

            WaitingClose { mut stream } => match stream.as_mut().poll_close(cx) {
                Poll::Ready(Ok(())) => Advanced::end(),
                Poll::Pending => Advanced::transition_to(WaitingClose { stream }),
                Poll::Ready(Err(error)) => Advanced::error(stream, error),
            },
        }
    }
}
