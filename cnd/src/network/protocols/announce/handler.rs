use crate::network::protocols::announce::protocol::{
    self, Confirmed, InboundConfig, OutboundConfig, ReplySubstream,
};
use libp2p::{
    core::upgrade::{InboundUpgrade, OutboundUpgrade},
    swarm::{
        KeepAlive, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent,
        ProtocolsHandlerUpgrErr, SubstreamProtocol,
    },
};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
};

/// Protocol handler for sending and receiving announce protocol messages.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Handler {
    /// Pending events to yield.
    #[derivative(Debug = "ignore")]
    events: Vec<HandlerEvent>,
    /// Whether the handler should keep the connection alive.
    keep_alive: KeepAlive,
    /// Queue of outbound substreams to open.
    dial_queue: VecDeque<OutboundConfig>,
}

/// Event produced by the `Handler`.
#[derive(Debug)]
pub enum HandlerEvent {
    /// This event created when a confirmation message containing a `swap_id` is
    /// received in response to an announce message containing a
    /// `swap_digest`. The Event contains both the swap id and
    /// the swap digest.
    ReceivedConfirmation(Confirmed),

    /// The event is created when a remote sends a `swap_digest`. The event
    /// contains a reply substream for the receiver to send back the
    /// `swap_id` that corresponds to the swap digest.
    AwaitingConfirmation(ReplySubstream<NegotiatedSubstream>),

    /// Failed to announce swap to peer.
    Error(Error),
}

impl Handler {
    /// Creates a new `Handler`.
    pub fn new() -> Self {
        Handler {
            events: vec![],
            keep_alive: KeepAlive::Yes,
            dial_queue: VecDeque::new(),
        }
    }
}

impl ProtocolsHandler for Handler {
    type InEvent = OutboundConfig;
    type OutEvent = HandlerEvent;
    type Error = Error;
    type InboundProtocol = InboundConfig;
    type OutboundProtocol = OutboundConfig;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(InboundConfig::default())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        sender: <Self::InboundProtocol as InboundUpgrade<NegotiatedSubstream>>::Output,
    ) {
        self.events.push(HandlerEvent::AwaitingConfirmation(sender))
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        confirmed: <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Output,
        _info: Self::OutboundOpenInfo,
    ) {
        self.events
            .push(HandlerEvent::ReceivedConfirmation(confirmed));
        self.keep_alive = KeepAlive::No;
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        self.keep_alive = KeepAlive::Yes;
        self.dial_queue.push_front(event);
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        err: ProtocolsHandlerUpgrErr<
            <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Error,
        >,
    ) {
        self.events.push(HandlerEvent::Error(Error::Upgrade(err)));
        self.keep_alive = KeepAlive::No;
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        self.keep_alive
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<
        ProtocolsHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            HandlerEvent,
            Self::Error,
        >,
    > {
        if !self.events.is_empty() {
            let event = self.events.remove(0);
            if let HandlerEvent::Error(err) = event {
                return Poll::Ready(ProtocolsHandlerEvent::Close(err));
            };
            return Poll::Ready(ProtocolsHandlerEvent::Custom(event));
        }

        if !self.dial_queue.is_empty() {
            let upgrade = self.dial_queue.remove(0).unwrap(); // Cannot fail, queue is not empty.
            return Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(upgrade),
                info: (),
            });
        }

        Poll::Pending
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("outbound upgrade failed")]
    Upgrade(#[from] ProtocolsHandlerUpgrErr<protocol::Error>),
}
