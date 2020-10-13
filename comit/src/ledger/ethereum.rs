use crate::ethereum::ChainId;
use fmt::Display;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
}

impl Ethereum {
    pub fn new(chain: ChainId) -> Self {
        Ethereum { chain_id: chain }
    }
}

impl Display for Ethereum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chain_id = u32::from(self.chain_id);
        let s = match chain_id {
            1 => "Mainnet",
            3 => "Ropsten",
            4 => "Rinkeby",
            5 => "Goerli",
            42 => "Kovan",
            _ => "Devnet",
        };

        write!(f, "{}", s)
    }
}

impl From<u32> for Ethereum {
    fn from(chain_id: u32) -> Self {
        Ethereum::new(chain_id.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn ethereum_serializes_as_expected() {
        let ledger = Ethereum::from(1);
        let want = r#"{"chain_id":1}"#.to_string();
        let got = serde_json::to_string(&ledger).expect("failed to serialize");

        assert_that(&got).is_equal_to(&want);
    }

    #[test]
    fn ethereum_serialization_roundtrip() {
        let ledger = Ethereum::from(1);
        let json = serde_json::to_string(&ledger).expect("failed to serialize");
        let rinsed: Ethereum = serde_json::from_str(&json).expect("failed to deserialize");

        assert_eq!(ledger, rinsed);
    }
}
