use crate::{
    asset,
    ethereum::ChainId,
    halbit, hbit, herc20,
    http_api::{DialInformation, Http},
    identity, ledger,
    network::{HalbitHerc20, HbitHerc20, Herc20Halbit, Herc20Hbit},
    storage::CreatedSwap,
    LocalSwapId, Role,
};
use chrono::Utc;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

impl From<Body<Herc20, Halbit>> for Herc20Halbit {
    fn from(body: Body<Herc20, Halbit>) -> Self {
        Self {
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.token_contract,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: body.beta.amount.0,
        }
    }
}

impl From<Body<Halbit, Herc20>> for HalbitHerc20 {
    fn from(body: Body<Halbit, Herc20>) -> Self {
        Self {
            lightning_cltv_expiry: body.alpha.cltv_expiry.into(),
            lightning_amount: body.alpha.amount.0,
            ethereum_absolute_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.token_contract,
        }
    }
}

impl From<Body<Herc20, Hbit>> for Herc20Hbit {
    fn from(body: Body<Herc20, Hbit>) -> Self {
        Self {
            ethereum_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.token_contract,
            bitcoin_expiry: body.beta.absolute_expiry.into(),
            bitcoin_amount: *body.beta.amount,
        }
    }
}

impl From<Body<Hbit, Herc20>> for HbitHerc20 {
    fn from(body: Body<Hbit, Herc20>) -> Self {
        Self {
            bitcoin_expiry: body.alpha.absolute_expiry.into(),
            bitcoin_amount: *body.alpha.amount,
            ethereum_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.token_contract,
        }
    }
}

impl<A, B> Body<A, B> {
    pub fn to_created_swap<CA, CB>(&self, swap_id: LocalSwapId) -> CreatedSwap<CA, CB>
    where
        CA: From<A>,
        CB: From<B>,
        Self: Clone,
    {
        let body = self.clone();

        let alpha = CA::from(body.alpha);
        let beta = CB::from(body.beta);

        let start_of_swap = Utc::now().naive_local();

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
            start_of_swap,
        }
    }
}

/// Data for the halbit protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Halbit {
    pub amount: Http<asset::Bitcoin>,
    pub identity: identity::Lightning,
    pub network: Http<ledger::Bitcoin>,
    pub cltv_expiry: u32,
}

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
