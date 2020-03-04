use crate::{
    frame::{self, OutboundRequest, Response, UnknownMandatoryHeaders, ValidatedInboundRequest},
    protocol::ComitProtocolConfig,
    substream::{self, Advance, Advanced},
    ComitHandlerEvent, Frame, Frames,
};
use futures::{
    channel::oneshot::{self, Canceled},
    task::{Poll, Waker},
};
use libp2p::swarm::{
    KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr, SubstreamProtocol,
};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Display,
    task::Context,
};

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct ComitHandler {
    #[derivative(Debug = "ignore")]
    inbound_substreams: Vec<substream::inbound::State>,
    #[derivative(Debug = "ignore")]
    outbound_substreams: Vec<substream::outbound::State>,

    to_send: Vec<PendingOutboundRequest>,

    #[derivative(Debug = "ignore")]
    current_task: Option<Waker>,

    known_headers: HashMap<String, HashSet<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed frame: ")]
    MalformedJson(#[from] frame::CodecError),
    #[error("dropped response: {0}")]
    DroppedResponseSender(#[from] Canceled),
    #[error("unknown mandatory header: {0:?}")]
    UnknownMandatoryHeader(UnknownMandatoryHeaders),
    #[error("unknown request type: {0}")]
    UnknownRequestType(String),
    #[error("unknown frame type")]
    UnknownFrameKind,
    #[error("unexpected frame")]
    UnexpectedFrame(Frame),
    #[error("malformed frame")]
    MalformedFrame(#[from] serde_json::Error),
    #[error("unexpected EOF")]
    UnexpectedEOF,
}

impl ComitHandler {
    pub fn new(known_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            known_headers,
            inbound_substreams: Vec::new(),
            outbound_substreams: Vec::new(),
            to_send: Vec::new(),
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

impl ProtocolsHandler for ComitHandler {
    type InEvent = ProtocolInEvent;
    type OutEvent = ProtocolOutEvent;
    type Error = Error;
    type InboundProtocol = ComitProtocolConfig;
    type OutboundProtocol = ComitProtocolConfig;
    type OutboundOpenInfo = ProtocolOutboundOpenInfo;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(ComitProtocolConfig {})
    }

    fn inject_fully_negotiated_inbound(&mut self, stream: Frames) {
        self.inbound_substreams
            .push(substream::inbound::State::WaitingMessage {
                stream: Box::pin(stream),
            });

        if let Some(waker) = self.current_task.take() {
            waker.wake()
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: Frames,
        outbound_open_info: Self::OutboundOpenInfo,
    ) {
        match outbound_open_info {
            ProtocolOutboundOpenInfo::Message(OutboundMessage::Request(
                PendingOutboundRequest { request, channel },
            )) => {
                self.outbound_substreams
                    .push(substream::outbound::State::WaitingSend {
                        frame: request.into(),
                        response_sender: channel,
                        stream: Box::pin(stream),
                    });
            }
        }

        if let Some(waker) = self.current_task.take() {
            waker.wake()
        }
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            ProtocolInEvent::Message(OutboundMessage::Request(request)) => {
                self.to_send.push(request)
            }
        }

        if let Some(waker) = self.current_task.take() {
            waker.wake()
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

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ComitHandlerEvent> {
        if let Some(request) = self.to_send.pop() {
            return Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(ComitProtocolConfig {}),
                info: ProtocolOutboundOpenInfo::Message(OutboundMessage::Request(request)),
            });
        }

        if let Some(result) =
            poll_substreams(&mut self.outbound_substreams, &self.known_headers, cx)
        {
            return result;
        }

        if let Some(result) = poll_substreams(&mut self.inbound_substreams, &self.known_headers, cx)
        {
            return result;
        }

        self.current_task = Some(cx.waker().clone());

        Poll::Pending
    }
}

fn poll_substreams<S>(
    substreams: &mut Vec<S>,
    known_headers: &HashMap<String, HashSet<String>>,
    cx: &mut Context<'_>,
) -> Option<Poll<ComitHandlerEvent>>
where
    S: Display + Advance,
{
    // We remove each element from `substreams` one by one and add them back.
    for n in (0..substreams.len()).rev() {
        let substream_state = substreams.swap_remove(n);

        let log_message = format!("transition from {}", substream_state);

        let Advanced { new_state, event } = substream_state.advance(known_headers, cx);

        if let Some(new_state) = new_state {
            tracing::trace!("{} to {}", log_message, new_state);
            substreams.push(new_state);
        }

        if let Some(event) = event {
            return Some(Poll::Ready(event));
        }
    }
    None
}
