use crate::network::{
    announce::{
        protocol,
        protocol::{Confirmed, InboundConfig, OutboundConfig},
    },
    protocols::announce::ReplySubstream,
    SwapDigest,
};
use libp2p::{
    core::upgrade::OutboundUpgrade,
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
    events: VecDeque<HandlerEvent>,
    /// Queue of outbound substreams to open.
    dial_queue: VecDeque<OutboundConfig>,
}

impl Default for Handler {
    fn default() -> Self {
        Handler {
            events: VecDeque::new(),
            dial_queue: VecDeque::new(),
        }
    }
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
    AwaitingConfirmation {
        swap_digest: SwapDigest,
        reply_substream: ReplySubstream<NegotiatedSubstream>,
    },

    /// Failed to announce swap to peer.
    Error(Error),
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
        (swap_digest, reply_substream): (SwapDigest, ReplySubstream<NegotiatedSubstream>),
    ) {
        self.events.push_back(HandlerEvent::AwaitingConfirmation {
            swap_digest,
            reply_substream,
        })
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        confirmed: Confirmed,
        _info: Self::OutboundOpenInfo,
    ) {
        self.events
            .push_back(HandlerEvent::ReceivedConfirmation(confirmed));
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        self.dial_queue.push_back(event);
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        err: ProtocolsHandlerUpgrErr<
            <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Error,
        >,
    ) {
        self.events
            .push_back(HandlerEvent::Error(Error::Upgrade(err)));
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    #[allow(clippy::type_complexity)]
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
        if let Some(event) = self.events.pop_front() {
            if let HandlerEvent::Error(err) = event {
                return Poll::Ready(ProtocolsHandlerEvent::Close(err));
            };
            return Poll::Ready(ProtocolsHandlerEvent::Custom(event));
        }

        if let Some(upgrade) = self.dial_queue.pop_front() {
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
