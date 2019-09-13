use crate::{
    frame::{Response, UnvalidatedInboundRequest},
    handler::{self, InboundMessage, PendingInboundRequest, ProtocolOutEvent},
    protocol::BamStream,
    substream::{Advance, Advanced, CloseStream},
    Frame, FrameType, IntoFrame,
};
use futures::sync::oneshot;
use libp2p_swarm::ProtocolsHandlerEvent;
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
                Ok(Async::Ready(Some(frame))) => match frame.frame_type {
                    FrameType::Request => {
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
                                    new_state: Some(WaitingUser { receiver, stream }),
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
                    FrameType::Response => {
                        Advanced::error(stream, handler::Error::UnexpectedFrame(frame))
                    }
                    FrameType::Unknown => Advanced::error(stream, handler::Error::UnknownFrameType),
                },
                Ok(Async::NotReady) => Advanced::transition_to(WaitingMessage { stream }),
                Ok(Async::Ready(None)) => Advanced::error(stream, handler::Error::UnexpectedEOF),
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
                Ok(Async::NotReady) => Advanced::transition_to(WaitingFlush { stream }),
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
