use crate::libp2p_bam::{
    protocol::BamProtocol,
    substream::{self, Advance, Advanced},
    BamHandlerEvent,
};
use bam::{
    frame::{
        JsonFrameCodec, OutboundRequest, Response, UnknownMandatoryHeaders, ValidatedInboundRequest,
    },
    Frame, IntoFrame,
};
use derivative::Derivative;
use futures::{
    sync::oneshot::{self, Canceled},
    task::Task,
    Async, Poll,
};
use libp2p::core::{
    protocols_handler::{KeepAlive, ProtocolsHandler, ProtocolsHandlerUpgrErr, SubstreamProtocol},
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
    inbound_substreams: Vec<substream::inbound::State<TSubstream>>,
    #[derivative(Debug = "ignore")]
    outbound_substreams: Vec<substream::outbound::State<TSubstream>>,

    #[derivative(Debug = "ignore")]
    current_task: Option<Task>,

    known_headers: HashMap<String, HashSet<String>>,
}

#[derive(Debug)]
pub enum Error {
    Stream(bam::frame::CodecError),
    DroppedResponseSender(Canceled),
    UnknownMandatoryHeader(UnknownMandatoryHeaders),
    UnknownRequestType(String),
    UnknownFrameType,
    UnexpectedFrame(Frame),
    MalformedFrame(serde_json::Error),
    UnexpectedEOF,
}

impl From<Canceled> for Error {
    fn from(e: Canceled) -> Self {
        Error::DroppedResponseSender(e)
    }
}

impl From<bam::frame::CodecError> for Error {
    fn from(e: bam::frame::CodecError) -> Self {
        Error::Stream(e)
    }
}

impl<TSubstream> BamHandler<TSubstream> {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            inbound_substreams: Vec::new(),
            outbound_substreams: Vec::new(),
            current_task: None,
        }
    }
}

#[derive(Debug)]
pub struct PendingOutboundRequest {
    pub request: OutboundRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingInboundRequest {
    pub request: ValidatedInboundRequest,
    pub channel: oneshot::Sender<Response>,
}

#[derive(Debug)]
pub struct PendingInboundResponse {
    pub response: Response,
    pub channel: oneshot::Sender<Response>,
}

/// Events that occur 'in' this node (as opposed to events from a peer node).
#[derive(Debug)]
pub enum ProtocolInEvent {
    Message(OutboundMessage),
}

/// Different kinds of `OutboundOpenInfo` that we may want to pass when emitted
/// an instance of `ProtocolsHandlerEvent::OutboundSubstreamRequest`.
#[derive(Debug)]
pub enum ProtocolOutboundOpenInfo {
    Message(OutboundMessage),
}

/// Events emitted after processing a message from the 'out'side of this node
/// i.e. from a peer
#[derive(Debug)]
pub enum ProtocolOutEvent {
    Message(InboundMessage),
    Error(Error),
}

#[derive(Debug)]
pub enum InboundMessage {
    Request(PendingInboundRequest),
    Response(PendingInboundResponse),
}

#[derive(Debug)]
pub enum OutboundMessage {
    Request(PendingOutboundRequest),
}

impl<TSubstream: AsyncRead + AsyncWrite> ProtocolsHandler for BamHandler<TSubstream> {
    type InEvent = ProtocolInEvent;
    type OutEvent = ProtocolOutEvent;
    type Error = bam::frame::CodecError;
    type Substream = TSubstream;
    type InboundProtocol = BamProtocol;
    type OutboundProtocol = BamProtocol;
    type OutboundOpenInfo = ProtocolOutboundOpenInfo;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(BamProtocol {})
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
    ) {
        self.inbound_substreams
            .push(substream::inbound::State::WaitingMessage { stream });

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: Framed<Negotiated<TSubstream>, JsonFrameCodec>,
        outbound_open_info: Self::OutboundOpenInfo,
    ) {
        match outbound_open_info {
            ProtocolOutboundOpenInfo::Message(OutboundMessage::Request(
                PendingOutboundRequest { request, channel },
            )) => {
                self.outbound_substreams
                    .push(substream::outbound::State::WaitingSend {
                        frame: request.into_frame(),
                        response_sender: channel,
                        stream,
                    });
            }
        }

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            ProtocolInEvent::Message(OutboundMessage::Request(request)) => {
                self.outbound_substreams
                    .push(substream::outbound::State::WaitingOpen { request });
            }
        }

        if let Some(task) = &self.current_task {
            task.notify()
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
        if let Some(result) = poll_substreams(&mut self.outbound_substreams, &self.known_headers) {
            return result;
        }

        if let Some(result) = poll_substreams(&mut self.inbound_substreams, &self.known_headers) {
            return result;
        }

        self.current_task = Some(futures::task::current());

        Ok(Async::NotReady)
    }
}

fn poll_substreams<S: Display + Advance>(
    substreams: &mut Vec<S>,
    known_headers: &HashMap<String, HashSet<String>>,
) -> Option<Poll<BamHandlerEvent, bam::frame::CodecError>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libp2p_bam::test_harness::{
        request_with_no_headers, setup_substream, setup_substream_with_json_codec, IntoEventStream,
        IntoFutureWithResponse, WaitForFrame,
    };
    use bam::Status;
    use futures::{Future, Sink, Stream};
    use libp2p::core::protocols_handler::ProtocolsHandlerEvent;
    use spectral::prelude::*;
    use tokio::codec::LinesCodec;

    #[test]
    fn given_inbound_substream_when_receiving_unknown_request_should_emit_unknown_request_type() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let (dialer, listener) = runtime.block_on(setup_substream_with_json_codec()).unwrap();
        let mut handler = BamHandler::new(HashMap::new());

        // given a substream
        handler.inject_fully_negotiated_inbound(listener);

        // when receiving a request
        let send = dialer.send(OutboundRequest::new("PING").into_frame());
        let _ = runtime.block_on(send).unwrap();

        let events = runtime
            .block_on(handler.into_event_stream().take(1).collect())
            .unwrap();

        // then we emit a BadInboundRequest
        matches::assert_matches!(
            events.get(0),
            Some(
                ProtocolsHandlerEvent::Custom(ProtocolOutEvent::Error(Error::UnknownRequestType(_))),
            )
        )
    }

    #[test]
    fn given_an_inbound_request_handler_sends_response() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let (dialer, listener) = runtime.block_on(setup_substream_with_json_codec()).unwrap();
        let mut handler = BamHandler::new(request_with_no_headers("PING"));

        // given an inbound substream
        handler.inject_fully_negotiated_inbound(listener);

        // and we receive a request
        let send = dialer.send(OutboundRequest::new("PING").into_frame());
        let dialer = runtime.block_on(send).unwrap();

        // and we provide an answer
        let future = handler.into_future_with_response(Response::new(Status::OK(127)));
        runtime.spawn(future);

        // then we send the response back to the dialer
        let response = runtime.block_on(dialer.wait_for_frame());

        assert_that(&response)
            .is_ok()
            .is_some()
            .is_equal_to(Response::new(Status::OK(127)).into_frame());
    }

    #[test]
    fn given_an_outbound_request_when_receiving_not_a_response_should_emit_unknown_frame_type() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let (dialer, listener) = runtime
            .block_on(setup_substream(
                JsonFrameCodec::default(),
                LinesCodec::new(),
            ))
            .unwrap();
        let mut handler = BamHandler::new(request_with_no_headers("PING"));

        // given an outbound substream
        let (sender, _receiver) = oneshot::channel();
        handler.inject_fully_negotiated_outbound(
            dialer,
            ProtocolOutboundOpenInfo::Message(OutboundMessage::Request(PendingOutboundRequest {
                request: OutboundRequest::new("PING"),
                channel: sender,
            })),
        );

        // when receiving something else than a response
        let send = listener
            .send(r#"{"type": "FOOBAR", "payload":{}}"#.to_owned())
            .map(|_| ())
            .map_err(|_| ());
        let _ = runtime.spawn(send);

        let events = runtime
            .block_on(handler.into_event_stream().take(1).collect())
            .unwrap();

        // then we emit a UnexpectedFrameType
        matches::assert_matches!(
            events.get(0),
            Some(ProtocolsHandlerEvent::Custom(ProtocolOutEvent::Error(
                Error::UnknownFrameType
            )))
        )
    }
}
