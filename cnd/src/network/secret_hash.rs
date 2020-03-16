use crate::{
    network::oneshot_protocol,
    swap_protocols::{rfc003::SecretHash, SwapId},
};
use serde::{Deserialize, Serialize};

/// The secret hash sharing protocol works in the following way:
///
/// - Dialer (Alice) writes the `Message` to the substream.
/// - Listener (Bob) reads the `Message` from the substream.

/// Data sent to peer in secret hash protocol.
#[derive(Clone, Copy, Deserialize, Debug, Serialize, PartialEq)]
pub struct Message {
    swap_id: SwapId,
    secret_hash: SecretHash,
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/secret_hash/1.0.0";
}
