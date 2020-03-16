use futures::{future::BoxFuture, AsyncRead, AsyncWrite};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{de::DeserializeOwned, Serialize};
use std::{io, iter, marker::PhantomData};

/// A trait for defining the message in a oneshot protocol.
pub trait Message {
    /// The identifier of the oneshot protocol.
    const INFO: &'static str;
}

/// Represents a prototype for an upgrade to handle the sender side of a oneshot
/// protocol.
///
/// This struct contains the message that should be sent to the other peer.
#[derive(Clone, Copy, Debug)]
pub struct OutboundConfig<M> {
    msg: M,
}

impl<M> UpgradeInfo for OutboundConfig<M>
where
    M: Message,
{
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(M::INFO.as_bytes())
    }
}

impl<C, M> OutboundUpgrade<C> for OutboundConfig<M>
where
    C: AsyncWrite + Unpin + Send + 'static,
    M: Serialize + Message + Send + 'static,
{
    type Output = ();
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
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
/// oneshot protocol.
///
/// The type parameter M is the message you are expecting to receive.
#[derive(Clone, Copy, Debug)]
pub struct InboundConfig<M> {
    msg_type: PhantomData<M>,
}

impl<M> Default for InboundConfig<M> {
    fn default() -> Self {
        Self {
            msg_type: PhantomData,
        }
    }
}

impl<M> UpgradeInfo for InboundConfig<M>
where
    M: Message,
{
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(M::INFO.as_bytes())
    }
}

impl<C, M> InboundUpgrade<C> for InboundConfig<M>
where
    C: AsyncRead + Unpin + Send + 'static,
    M: DeserializeOwned + Message,
{
    type Output = M;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading inbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let info = M::deserialize(&mut de)?;

            Ok(info)
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
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
    struct DummyMessage {
        content: u32,
    }

    impl Message for DummyMessage {
        const INFO: &'static str = "/foo/bar/test/1.0.0";
    }

    #[tokio::test]
    async fn correct_transfer() {
        let sent_msg = DummyMessage { content: 42 };

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

                let config = OutboundConfig { msg: sent_msg };
                upgrade::apply_outbound(conn, config, upgrade::Version::V1)
                    .await
                    .unwrap();
            }
        });

        let conn = MemoryTransport.dial(listener_addr).unwrap().await.unwrap();

        let config = InboundConfig::<DummyMessage>::default();
        let received_msg = upgrade::apply_inbound(conn, config).await.unwrap();

        assert_eq!(received_msg, sent_msg)
    }
}
