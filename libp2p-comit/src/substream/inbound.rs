use crate::{
    frame::{Response, UnvalidatedInboundRequest},
    handler::{self, InboundMessage, PendingInboundRequest, ProtocolOutEvent},
    protocol::Frames,
    substream::{Advance, Advanced, CloseStream},
    Frame, FrameKind,
};
use futures::{channel::oneshot, task::Poll, Future, Sink, Stream};
use libp2p::swarm::ProtocolsHandlerEvent;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    task::Context,
};

#[derive(strum_macros::Display)]
#[allow(missing_debug_implementations)]
/// States of an inbound substream i.e. from peer node to us.
pub enum State {
    /// Waiting for a request from the remote.
    WaitingMessage { stream: Pin<Box<Frames>> },
    /// Waiting for the user to send the response back to us.
    WaitingUser {
        receiver: Pin<Box<oneshot::Receiver<Response>>>,
        stream: Pin<Box<Frames>>,
    },
    /// Waiting to send an answer back to the remote.
    WaitingSend {
        msg: Frame,
        stream: Pin<Box<Frames>>,
    },
    /// Waiting to flush an answer back to the remote.
    WaitingFlush { stream: Pin<Box<Frames>> },
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
            WaitingMessage { mut stream } => match stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.kind {
                    FrameKind::Request => {
                        let request =
                            serde_json::from_value::<UnvalidatedInboundRequest>(frame.payload)
                                .map_err(handler::Error::MalformedFrame)
                                .and_then(|request| {
                                    known_headers
                                        .get(request.request_type())
                                        .ok_or_else(|| {
                                            handler::Error::UnknownRequestType(
                                                request.request_type().to_owned(),
                                            )
                                        })
                                        .and_then(|known_headers| {
                                            request
                                                .ensure_no_unknown_mandatory_headers(known_headers)
                                                .map_err(handler::Error::UnknownMandatoryHeader)
                                        })
                                });

                        match request {
                            Ok(request) => {
                                let (sender, receiver) = oneshot::channel();
                                Advanced {
                                    new_state: Some(WaitingUser {
                                        receiver: Box::pin(receiver),
                                        stream,
                                    }),
                                    event: Some(ProtocolsHandlerEvent::Custom(
                                        ProtocolOutEvent::Message(InboundMessage::Request(
                                            PendingInboundRequest {
                                                request,
                                                channel: sender,
                                            },
                                        )),
                                    )),
                                }
                            }
                            Err(error) => Advanced::error(stream, error),
                        }
                    }
                    FrameKind::Response => {
                        Advanced::error(stream, handler::Error::UnexpectedFrame(frame))
                    }
                    FrameKind::Unknown => Advanced::error(stream, handler::Error::UnknownFrameKind),
                },
                Poll::Ready(Some(Err(error))) => {
                    Advanced::error(stream, handler::Error::MalformedJson(error))
                }
                Poll::Pending => Advanced::transition_to(WaitingMessage { stream }),
                Poll::Ready(None) => Advanced::error(stream, handler::Error::UnexpectedEOF),
            },
            WaitingUser {
                mut receiver,
                stream,
            } => match receiver.as_mut().poll(cx) {
                Poll::Ready(Ok(response)) => WaitingSend {
                    msg: response.into(),
                    stream,
                }
                .advance(known_headers, cx),
                Poll::Pending => Advanced::transition_to(WaitingUser { receiver, stream }),
                Poll::Ready(Err(error)) => Advanced::error(stream, error),
            },
            WaitingSend { msg, mut stream } => match stream.as_mut().start_send(msg) {
                Ok(()) => WaitingFlush { stream }.advance(known_headers, cx),
                Err(error) => Advanced::error(stream, error),
            },
            WaitingFlush { mut stream } => match stream.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => Advanced::transition_to(WaitingClose { stream }),
                Poll::Ready(Err(error)) => Advanced::error(stream, error),
                Poll::Pending => Advanced::transition_to(WaitingFlush { stream }),
            },
            WaitingClose { mut stream } => match stream.as_mut().poll_close(cx) {
                Poll::Ready(Ok(())) => Advanced::end(),
                Poll::Pending => Advanced::transition_to(WaitingClose { stream }),
                Poll::Ready(Err(error)) => Advanced::error(stream, error),
            },
        }
    }
}
