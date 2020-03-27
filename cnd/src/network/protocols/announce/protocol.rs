use crate::{network::protocols::announce::SwapDigest, swap_protocols::SwapId};
use futures::prelude::*;
use libp2p::core::upgrade::{self, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::Deserialize;
use std::{io, iter, pin::Pin};

const INFO: &str = "/comit/swap/announce/1.0.0";

/// Configuration for an upgrade to the `Announce` protocol on the outbound
/// side.
#[derive(Debug, Clone)]
pub struct OutboundConfig {
    pub swap_digest: SwapDigest,
}

impl OutboundConfig {
    pub fn new(swap_digest: SwapDigest) -> Self {
        OutboundConfig { swap_digest }
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
    type Output = Confirmed;
    type Error = Error;
    type Future = UpgradeFuture<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let bytes = serde_json::to_vec(&self.swap_digest)?;
            upgrade::write_one(&mut socket, &bytes).await?;
            socket.close().await?;

            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_id = SwapId::deserialize(&mut de)?;
            tracing::trace!("Received: {}", swap_id);

            Ok(Confirmed {
                swap_digest: self.swap_digest.clone(),
                swap_id,
            })
        })
    }
}

#[derive(Debug)]
pub struct Confirmed {
    pub swap_digest: SwapDigest,
    pub swap_id: SwapId,
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

impl<C> InboundUpgrade<C> for InboundConfig
where
    C: AsyncRead + Unpin + Send + 'static,
{
    type Output = ReplySubstream<C>;
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
            let swap_digest = SwapDigest::deserialize(&mut de)?;
            Ok(ReplySubstream {
                io: socket,
                swap_digest,
            })
        })
    }
}

/// The substream on which a reply is expected to be sent.
#[derive(Debug)]
pub struct ReplySubstream<T> {
    pub io: T,
    pub swap_digest: SwapDigest,
}

impl<T> ReplySubstream<T>
where
    T: AsyncWrite + Unpin,
{
    /// Sends back the requested information on the substream i.e., the
    /// `swap_id`.
    ///
    /// Consumes the substream, returning a reply future that resolves
    /// when the reply has been sent on the underlying connection.
    pub async fn send(mut self, swap_id: SwapId) -> Result<(), Error> {
        tracing::trace!("Sending: {}", swap_id);

        let bytes = serde_json::to_vec(&swap_id)?;
        upgrade::write_one(&mut self.io, &bytes).await?;

        Ok(())
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
