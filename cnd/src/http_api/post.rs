use crate::{
    asset,
    ethereum::ChainId,
    halbit, hbit, herc20,
    http_api::{DialInformation, Http, PostBody},
    identity, ledger,
    network::{HalbitHerc20, HbitHerc20, Herc20Halbit, Herc20Hbit},
    storage::CreatedSwap,
    LocalSwapId, Role,
};
use chrono::Utc;

/// Data for the herc20 protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Herc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: ChainId,
    pub token_contract: identity::Ethereum,
    pub absolute_expiry: u32,
}

/// Data for the hbit protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Hbit {
    pub amount: Http<asset::Bitcoin>,
    pub final_identity: Http<bitcoin::Address>,
    pub network: Http<bitcoin::Network>,
    pub absolute_expiry: u32,
}

impl From<Halbit> for halbit::CreatedSwap {
    fn from(p: Halbit) -> Self {
        halbit::CreatedSwap {
            asset: *p.amount,
            identity: p.identity,
            network: *p.network,
            cltv_expiry: p.cltv_expiry,
        }
    }
}

impl From<Herc20> for herc20::CreatedSwap {
    fn from(p: Herc20) -> Self {
        herc20::CreatedSwap {
            asset: asset::Erc20::new(p.token_contract, p.amount),
            identity: p.identity,
            chain_id: p.chain_id,
            absolute_expiry: p.absolute_expiry,
        }
    }
}

impl From<Hbit> for hbit::CreatedSwap {
    fn from(p: Hbit) -> Self {
        hbit::CreatedSwap {
            amount: *p.amount,
            final_identity: p.final_identity.0,
            network: p.network.0.into(),
            absolute_expiry: p.absolute_expiry,
        }
    }
}
