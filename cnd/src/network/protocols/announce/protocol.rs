use crate::{network::protocols::announce::SwapDigest, swap_protocols::SwapId};
use futures::prelude::*;
use libp2p::core::upgrade::{self, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::Deserialize;
use std::{fmt, io, iter, pin::Pin};

const INFO: &'static str = "/comit/swap/announce/1.0.0";

/// Configuration for an upgrade to the `Announce` protocol on the outbound
/// side.
#[derive(Debug, Clone)]
pub struct OutboundConfig {
    swap_digest: SwapDigest,
}

impl UpgradeInfo for OutboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<C> OutboundUpgrade<C> for OutboundConfig
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = SwapId;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let bytes = serde_json::to_vec(&self.swap_digest)?;
            upgrade::write_one(&mut socket, &bytes).await?;
            // FIXME: Is this correct (do we need a close of the write end for some reason
            // before reading)?
            socket.close().await?;

            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_id = SwapId::deserialize(&mut de)?;
            tracing::trace!("Received: {}", swap_id);

            Ok(swap_id)
        })
    }
}

/// Configuration for an upgrade to the `Announce` protocol on the inbound side.
#[derive(Debug, Clone)]
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
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: C, info: Self::Info) -> Self::Future {
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
pub struct ReplySubstream<T> {
    io: T,
    swap_digest: SwapDigest,
}

impl<T> fmt::Debug for ReplySubstream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReplySubstream").finish()
    }
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
    pub async fn send(mut self, swap_id: SwapId) -> impl Future<Output = Result<(), Error>> {
        tracing::trace!("Sending: {}", swap_id);
        async move {
            // TODO: Remove unwrap.
            let bytes = serde_json::to_vec(&swap_id).unwrap();
            Ok(upgrade::write_one(&mut self.io, &bytes).await?)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{IdentifyInfo, IdentifyProtocolConfig, RemoteInfo};
    use futures::{channel::oneshot, prelude::*};
    use libp2p_core::{
        identity,
        upgrade::{self, apply_inbound, apply_outbound},
        Transport,
    };
    use libp2p_tcp::TcpConfig;

    #[tokio::test]
    async fn correct_transfer() {
        // We open a server and a client, send info from the server to the client, and
        // check that they were successfully received.
        let send_swap_digest = vec![0u8, 1u8, 2u8];
        let send_swap_id = SwapId::from_str("ad2652ca-ecf2-4cc6-b35c-b4351ac28a34").unwrap();

        let (tx, rx) = oneshot::channel();

        let bg_task = tokio::task::spawn(async move {
            let transport = TcpConfig::new();

            let mut listener = transport
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();

            let addr = listener
                .next()
                .await
                .expect("some event")
                .expect("no error")
                .into_new_address()
                .expect("listen address");
            tx.send(addr).unwrap();

            let socket = listener
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_upgrade()
                .unwrap()
                .0
                .await
                .unwrap();
            let sender = apply_inbound(socket, InboundConfig::default())
                .await
                .unwrap();
            let receive_swad_digest = sender.swap_digest;

            assert_eq!(send_swap_digest, receive_swap_digest);

            sender.send(swap_id).await.unwrap();
        });

        let transport = TcpConfig::new();

        let socket = transport.dial(rx.await.unwrap()).unwrap().await.unwrap();
        let receive_swap_id = apply_outbound(
            socket,
            OutboundConfig { send_swap_digest },
            upgrade::Version::V1,
        )
        .await
        .unwrap();

        assert_eq!(send_swap_id, receive_swap_id)
    }
}
