use crate::{
    network::{
        protocols::{orderbook::OrderId, ReplySubstream},
        swap_digest::SwapDigest,
    },
    SharedSwapId,
};
use futures::prelude::*;
use libp2p::core::upgrade::{self, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{Deserialize, Serialize};
use std::{io, iter, pin::Pin};

const INFO: &str = "/comit/dbook/take_order/1.0.0";

/// Outbound message containing the order id
#[derive(Debug, Clone)]
pub struct OutboundConfig {
    // why cant the swap_digest be the order_id?
    pub order_id: OrderId,
    pub swap_digest: SwapDigest,
}

impl OutboundConfig {
    pub fn new(order_id: OrderId, swap_digest: SwapDigest) -> Self {
        OutboundConfig {
            order_id,
            swap_digest,
        }
    }
}

impl UpgradeInfo for OutboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

type UpgradeFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

impl<C> OutboundUpgrade<C> for OutboundConfig
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = OrderConfirmed;
    type Error = Error;
    type Future = UpgradeFuture<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let message = InboundMessage {
                order_id: self.order_id,
                swap_digest: self.swap_digest.clone(),
            };
            let bytes = serde_json::to_vec(&message)?;
            upgrade::write_one(&mut socket, &bytes).await?;
            socket.close().await?;

            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let shared_swap_id = SharedSwapId::deserialize(&mut de)?;

            let order_confirmation = OrderConfirmed {
                swap_digest: self.swap_digest.clone(),
                swap_id: shared_swap_id,
            };
            tracing::trace!("Received: {}", order_confirmation.swap_id);

            Ok(order_confirmation)
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct OrderConfirmed {
    pub swap_digest: SwapDigest,
    pub swap_id: SharedSwapId,
}

#[derive(Debug)]
pub struct InboundTakeOrderRequest<T> {
    pub order_id: OrderId,
    pub reply_substream: ReplySubstream<T>,
}

/// Configuration for an upgrade to the `Announce` protocol on the inbound side.
#[derive(Debug, Clone, Copy)]
pub struct InboundConfig {}

impl Default for InboundConfig {
    fn default() -> Self {
        InboundConfig {}
    }
}

impl UpgradeInfo for InboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InboundMessage {
    order_id: OrderId,
    swap_digest: SwapDigest,
}

impl<C> InboundUpgrade<C> for InboundConfig
where
    C: AsyncRead + Unpin + Send + 'static,
{
    type Output = InboundTakeOrderRequest<C>;
    type Error = Error;
    type Future = UpgradeFuture<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading inbound connection for {}",
            String::from_utf8_lossy(info)
        );

        Box::pin(async move {
            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let inbound = InboundMessage::deserialize(&mut de)?;
            Ok(InboundTakeOrderRequest {
                order_id: inbound.order_id,
                reply_substream: ReplySubstream {
                    io: socket,
                    swap_digest: inbound.swap_digest,
                },
            })
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read a message from the socket")]
    Read(#[from] upgrade::ReadOneError),
    #[error("failed to write the message to the socket")]
    Write(#[from] io::Error),
    #[error("failed to serialize/deserialize the message")]
    Serde(#[from] serde_json::Error),
}
