use crate::{ethereum, identity, network::oneshot_protocol, SharedSwapId};
use serde::{Deserialize, Serialize};
use serdebug::SerDebug;

/// The message for the Ethereum identity sharing protocol.
#[derive(Clone, Copy, Deserialize, SerDebug, Serialize)]
pub struct Message {
    pub swap_id: SharedSwapId,
    /// An Ethereum address, serialized with a `0x` prefix as per convention in
    /// the Ethereum ecosystem.
    #[serde(with = "ethereum::serde_hex_data")]
    pub address: [u8; 20],
}

impl Message {
    pub fn new(swap_id: SharedSwapId, address: identity::Ethereum) -> Self {
        Self {
            swap_id,
            address: address.into(),
        }
    }
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/identity/ethereum/1.0.0";
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn serialization_format_stability_test() {
        let given = Message {
            swap_id: SharedSwapId::nil(),
            address: [0u8; 20],
        };

        let actual = serde_json::to_string(&given);

        assert_that(&actual).is_ok_containing(r#"{"swap_id":"00000000-0000-0000-0000-000000000000","address":"0x0000000000000000000000000000000000000000"}"#.to_owned())
    }
}
