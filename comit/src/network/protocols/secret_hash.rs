use crate::{network::oneshot_protocol, SecretHash, SharedSwapId};
use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, Strict};
use serdebug::SerDebug;

/// The message for the secret hash sharing protocol.
#[derive(Clone, Copy, Deserialize, SerDebug, Serialize)]
pub struct Message {
    pub swap_id: SharedSwapId,
    /// A SHA-256 hash, serialized as hex without a `0x` prefix.
    #[serde(with = "SerHex::<Strict>")]
    pub secret_hash: [u8; 32],
}

impl Message {
    pub fn new(swap_id: SharedSwapId, secret_hash: SecretHash) -> Self {
        Self {
            swap_id,
            secret_hash: secret_hash.into_raw(),
        }
    }
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/secret_hash/1.0.0";
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn serialization_format_stability_test() {
        let given = Message {
            swap_id: SharedSwapId::nil(),
            secret_hash: [0u8; 32],
        };

        let actual = serde_json::to_string(&given);

        assert_that(&actual).is_ok_containing(r#"{"swap_id":"00000000-0000-0000-0000-000000000000","secret_hash":"0000000000000000000000000000000000000000000000000000000000000000"}"#.to_owned())
    }
}
