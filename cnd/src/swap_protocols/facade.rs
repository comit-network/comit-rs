use crate::{
    asset,
    db::CreatedSwap,
    identity,
    network::{comit_ln, protocols::announce::SwapDigest, DialInformation, Swarm},
    swap_protocols::{halight, LedgerStates, LocalSwapId, Role},
    timestamp::Timestamp,
};
use digest::{Digest, IntoDigestInput};
use std::sync::Arc;

/// This represent the information available on a swap
/// before communication with the other node has started
#[derive(Clone, Digest, Debug)]
#[digest(hash = "SwapDigest")]
pub struct HanEtherereumHalightBitcoinCreateSwapParams {
    #[digest(ignore)]
    pub role: Role,
    #[digest(ignore)]
    pub peer: DialInformation,
    #[digest(ignore)]
    pub ethereum_identity: EthereumIdentity,
    #[digest(prefix = "2001")]
    pub ethereum_absolute_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub ethereum_amount: asset::Ether,
    #[digest(ignore)]
    pub lightning_identity: identity::Lightning,
    #[digest(prefix = "3001")]
    pub lightning_cltv_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub lightning_amount: asset::Bitcoin,
}

impl IntoDigestInput for asset::Bitcoin {
    fn into_digest_input(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl IntoDigestInput for asset::Ether {
    fn into_digest_input(self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EthereumIdentity(identity::Ethereum);

impl From<identity::Ethereum> for EthereumIdentity {
    fn from(inner: identity::Ethereum) -> Self {
        EthereumIdentity(inner)
    }
}

impl From<EthereumIdentity> for identity::Ethereum {
    fn from(outer: EthereumIdentity) -> Self {
        outer.0
    }
}

impl IntoDigestInput for Timestamp {
    fn into_digest_input(self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
pub struct Facade {
    pub swarm: Swarm,
    // We currently only support Han-HALight, therefor 'alpha' is Ethereum and 'beta' is Lightning.
    pub alpha_ledger_states: Arc<LedgerStates>,
    pub beta_ledger_states: Arc<halight::States>,
}

impl Facade {
    pub async fn save<A, B>(&self, _: CreatedSwap<A, B>) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        self.swarm.initiate_communication(id, swap_params).await
    }

    pub async fn get_finalized_swap(&self, id: LocalSwapId) -> Option<comit_ln::FinalizedSwap> {
        self.swarm.get_finalized_swap(id).await
    }
}
