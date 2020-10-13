use crate::{identity, network::oneshot_protocol, SharedSwapId};
use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, Strict};
use serdebug::SerDebug;

/// The message for the Lightning identity sharing protocol.
#[derive(Clone, Copy, Deserialize, Serialize, SerDebug)]
pub struct Message {
    pub swap_id: SharedSwapId,
    /// A Lightning node identifier is a compressed secp256k1 public key,
    /// serialized without a `0x` prefix.
    // TODO: Replace with #[serde(with = "hex")] on Rust 1.47 and remove serde-hex from dependencies
    #[serde(with = "SerHex::<Strict>")]
    pub pubkey: [u8; 33],
}

impl Message {
    pub fn new(swap_id: SharedSwapId, pubkey: identity::Lightning) -> Self {
        Self {
            swap_id,
            pubkey: bitcoin::PublicKey::from(pubkey).key.serialize(),
        }
    }
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/identity/lightning/1.0.0";
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn serialization_format_stability_test() {
        let given = Message {
            swap_id: SharedSwapId::nil(),
            pubkey: [0u8; 33],
        };

        let actual = serde_json::to_string(&given);

        assert_that(&actual).is_ok_containing(r#"{"swap_id":"00000000-0000-0000-0000-000000000000","pubkey":"000000000000000000000000000000000000000000000000000000000000000000"}"#.to_owned())
    }
}
