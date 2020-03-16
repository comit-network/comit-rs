use crate::swap_protocols::SwapId;
use futures::{future::BoxFuture, AsyncRead, AsyncWrite};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{Deserialize, Serialize};
use std::{io, iter};
use tracing::trace;

const INFO: &str = "/comit/swap/finalize/1.0.0";

/// The finalize protocol works in the following way:
///
/// - Dialer (Alice) writes the `Message` to the substream.
/// - Listener (Bob) reads the `Message` from the substream.

/// Data sent to peer in finalize protocol.
#[derive(Clone, Copy, Deserialize, Debug, Serialize, PartialEq)]
pub struct Message {
    swap_id: SwapId,
}

/// Represents a prototype for an upgrade to handle the sender side of the
/// finalize protocol.  Config contains the `Message`, once the outbound upgrade
/// is complete peer node has been sent the message.
#[derive(Clone, Copy, Debug)]
pub struct OutboundProtocolConfig {
    msg: Message,
}

impl UpgradeInfo for OutboundProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<C> OutboundUpgrade<C> for OutboundProtocolConfig
where
    C: AsyncWrite + Unpin + Send + 'static,
{
    type Output = ();
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        trace!(
            "Upgrading outbound connection: {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let bytes = serde_json::to_vec(&self.msg)?;
            upgrade::write_one(&mut socket, &bytes).await?;

            Ok(())
        })
    }
}

/// Represents a prototype for an upgrade to handle the receiver side of the
/// finalize protocol.
#[derive(Clone, Copy, Debug)]
pub struct InboundProtocolConfig;

impl Default for InboundProtocolConfig {
    fn default() -> Self {
        Self {}
    }
}

impl UpgradeInfo for InboundProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<C> InboundUpgrade<C> for InboundProtocolConfig
where
    C: AsyncRead + Unpin + Send + 'static,
{
    type Output = Message;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        trace!(
            "Upgrading inbound connection: {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let info = Message::deserialize(&mut de)?;

            Ok(info)
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("read from socket: ")]
    Read(#[from] upgrade::ReadOneError),
    #[error("write to socket: ")]
    Write(#[from] io::Error),
    #[error("serde: ")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::prelude::*;
    use libp2p::core::{
        multiaddr::multiaddr,
        transport::{memory::MemoryTransport, ListenerEvent, Transport},
        upgrade,
    };
    use rand::{thread_rng, Rng};
    use std::str::FromStr;

    #[tokio::test]
    async fn correct_transfer() {
        let send_msg = Message {
            swap_id: SwapId::from_str("ad2652ca-ecf2-4cc6-b35c-b4351ac28a34").unwrap(),
        };

        let mem_addr = multiaddr![Memory(thread_rng().gen::<u64>())];
        let mut listener = MemoryTransport.listen_on(mem_addr).unwrap();

        let listener_addr =
            if let Some(Some(Ok(ListenerEvent::NewAddress(a)))) = listener.next().now_or_never() {
                a
            } else {
                panic!("MemoryTransport not listening on an address!");
            };

        tokio::task::spawn({
            async move {
                let listener_event = listener.next().await.unwrap();
                let (listener_upgrade, _) = listener_event.unwrap().into_upgrade().unwrap();
                let conn = listener_upgrade.await.unwrap();

                let config = OutboundProtocolConfig { msg: send_msg };
                upgrade::apply_outbound(conn, config, upgrade::Version::V1)
                    .await
                    .unwrap();
            }
        });

        let conn = MemoryTransport.dial(listener_addr).unwrap().await.unwrap();

        let config = InboundProtocolConfig {};
        let received_msg = upgrade::apply_inbound(conn, config).await.unwrap();

        assert_eq!(received_msg, send_msg)
    }
}
