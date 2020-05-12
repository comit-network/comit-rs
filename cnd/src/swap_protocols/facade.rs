use crate::{
    asset,
    db::CreatedSwap,
    identity,
    network::{comit_ln, protocols::announce::SwapDigest, DialInformation, Swarm},
    swap_protocols::{halight, LedgerStates, LocalSwapId, Role},
    timestamp::{RelativeTime, Timestamp},
};
use digest::Digest;
use std::sync::Arc;

/// This represent the information available on a swap
/// before communication with the other node has started
#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "SwapDigest")]
pub struct Herc20HalightBitcoinCreateSwapParams {
    #[digest(ignore)]
    pub role: Role,
    #[digest(ignore)]
    pub peer: DialInformation,
    #[digest(ignore)]
    pub ethereum_identity: identity::Ethereum,
    #[digest(prefix = "2001")]
    pub ethereum_absolute_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub ethereum_amount: asset::Erc20Quantity,
    #[digest(ignore)]
    pub token_contract: identity::Ethereum,
    #[digest(ignore)]
    pub lightning_identity: identity::Lightning,
    #[digest(prefix = "3001")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "3002")]
    pub lightning_amount: asset::Bitcoin,
}

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
pub struct Facade {
    pub swarm: Swarm,
    // We currently only support Han-HALight, therefor 'alpha' is Ethereum and 'beta' is Lightning.
    pub alpha_ledger_states: Arc<LedgerStates>,
    pub halight_states: Arc<halight::States>,
}

impl Facade {
    pub async fn save<A, B>(&self, _: CreatedSwap<A, B>) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        self.swarm.initiate_communication(id, swap_params).await
    }

    pub async fn get_finalized_swap(&self, id: LocalSwapId) -> Option<comit_ln::FinalizedSwap> {
        self.swarm.get_finalized_swap(id).await
    }

    pub async fn get_created_swap(
        &self,
        id: LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        self.swarm.get_created_swap(id).await
    }
}
