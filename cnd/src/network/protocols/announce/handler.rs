use crate::{
    network::protocols::announce::protocol::{self, InboundConfig, OutboundConfig, ReplySubstream},
    swap_protocols::SwapId,
};
use libp2p::{
    core::upgrade::{InboundUpgrade, OutboundUpgrade, ReadOneError},
    swarm::{
        KeepAlive, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent,
        ProtocolsHandlerUpgrErr, SubstreamProtocol,
    },
};
use std::task::{Context, Poll};

/// Protocol handler for sending and receiving announce protocol messages.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Handler {
    /// Pending events to yield.
    #[derivative(Debug = "ignore")]
    events: Vec<HandlerEvent>,

    /// Whether the handler should keep the connection alive.
    keep_alive: KeepAlive,
}

/// Event produced by the `Handler`.
#[derive(Debug)]
pub enum HandlerEvent {
    /// Node (Alice) announces the swap by way of the protocol upgrade - result
    /// of the successful application of this upgrade is the SwapId sent back
    /// from peer (Bob).
    Announce(SwapId),

    /// Node (Bob) received the announced swap (inc. swap_digest) from peer
    /// (Alice).
    Announced(ReplySubstream<NegotiatedSubstream>),

    /// Failed to announce swap to peer.
    AnnounceError(Error),
}

impl Handler {
    /// Creates a new `Handler`.
    pub fn new() -> Self {
        Handler {
            events: vec![],
            keep_alive: KeepAlive::Yes,
        }
    }
}

impl ProtocolsHandler for Handler {
    type InEvent = ();
    type OutEvent = HandlerEvent;
    type Error = ReadOneError;
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
        self.events.push(HandlerEvent::Announced(sender))
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        swap_id: <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Output,
        _info: Self::OutboundOpenInfo,
    ) {
        self.events.push(HandlerEvent::Announce(swap_id));
        self.keep_alive = KeepAlive::No;
    }

    fn inject_event(&mut self, _: Self::InEvent) {}

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        err: ProtocolsHandlerUpgrErr<
            <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Error,
        >,
    ) {
        self.events
            .push(HandlerEvent::AnnounceError(Error::Upgrade(err)));
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
            return Poll::Ready(ProtocolsHandlerEvent::Custom(self.events.remove(0)));
        }

        // if let Some(event) = self.events.pop_front() {
        //     return Poll::Ready(event);
        // }

        Poll::Pending
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("outbound upgrade failed")]
    Upgrade(#[from] ProtocolsHandlerUpgrErr<protocol::Error>),
}
