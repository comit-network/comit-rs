pub mod announce;
pub mod bitcoin_identity;
pub mod ethereum_identity;
pub mod finalize;
pub mod lightning_identity;
pub mod secret_hash;
pub mod setup_swap;

use super::swap_digest::SwapDigest;
use futures::prelude::*;
use libp2p::core::upgrade::{self};
use serde::Serialize;
use std::{fmt::Display, io};

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
    pub async fn send<U: Serialize + Display>(mut self, msg: U) -> Result<(), ReplyError> {
        tracing::trace!("Sending: {}", &msg);

        let bytes = serde_json::to_vec(&msg)?;
        upgrade::write_one(&mut self.io, &bytes).await?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplyError {
    #[error("failed to read a message from the socket")]
    Read(#[from] upgrade::ReadOneError),
    #[error("failed to write the message to the socket")]
    Write(#[from] io::Error),
    #[error("failed to serialize/deserialize the message")]
    Serde(#[from] serde_json::Error),
}
