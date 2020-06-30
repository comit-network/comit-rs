use crate::network::protocols::{
    orderbook::OrderId,
    take_order::{
        protocol,
        protocol::{InboundConfig, OrderConfirmed, OutboundConfig},
    },
    ReplySubstream,
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
    ReceivedTakeOrderResponse(OrderConfirmed),
    ReceivedTakeOrderRequest {
        order_id: OrderId,
        reply_substream: ReplySubstream<NegotiatedSubstream>,
    },
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
        inbound: <Self::InboundProtocol as InboundUpgrade<NegotiatedSubstream>>::Output,
    ) {
        self.events
            .push_back(HandlerEvent::ReceivedTakeOrderRequest {
                order_id: inbound.order_id,
                reply_substream: inbound.reply_substream,
            });
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        confirmed_order: <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Output,
        _info: Self::OutboundOpenInfo,
    ) {
        self.events
            .push_back(HandlerEvent::ReceivedTakeOrderResponse(confirmed_order));
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
