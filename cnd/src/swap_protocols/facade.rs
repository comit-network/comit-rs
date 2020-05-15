use crate::{
    asset,
    db::{CreatedSwap, Save, Sqlite},
    identity,
    network::{
        comit_ln, protocols::announce::SwapDigest, DialInformation, InitCommunication, Swarm,
    },
    swap_protocols::{halight, hbit, herc20, LedgerStates, LocalSwapId, Role},
    timestamp::{RelativeTime, Timestamp},
};
use digest::{Digest, ToDigestInput};
use std::sync::Arc;

/// This represents the information available on a swap
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

/// This represents the information available on a swap
/// before communication with the other node has started
#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "SwapDigest")]
pub struct HbitHerc20SwapParams {
    #[digest(ignore)]
    pub role: Role,
    #[digest(ignore)]
    pub peer: DialInformation,
    #[digest(ignore)]
    pub bitcoin_identity: identity::Bitcoin,
    #[digest(prefix = "2001")]
    pub bitcoin_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub bitcoin_amount: asset::Bitcoin,
    #[digest(ignore)]
    pub ethereum_identity: identity::Ethereum,
    #[digest(prefix = "3001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(ignore)]
    pub token_contract: identity::Ethereum,
}

impl From<CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>> for HbitHerc20SwapParams {
    fn from(swap: CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>) -> Self {
        let peer = DialInformation {
            peer_id: swap.peer,
            address_hint: None,
        };

        Self {
            role: swap.role,
            peer,
            bitcoin_identity: swap.alpha.identity,
            bitcoin_expiry: swap.alpha.absolute_expiry.into(),
            bitcoin_amount: swap.alpha.amount,
            ethereum_identity: swap.beta.identity,
            ethereum_expiry: swap.beta.absolute_expiry.into(),
            erc20_amount: swap.beta.amount,
            token_contract: swap.beta.token_contract,
        }
    }
}

#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "SwapDigest")]
pub struct Herc20HbitSwapParams {
    #[digest(ignore)]
    pub role: Role,
    #[digest(ignore)]
    pub peer: DialInformation,
    #[digest(ignore)]
    pub ethereum_identity: identity::Ethereum,
    #[digest(prefix = "2001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(ignore)]
    pub token_contract: identity::Ethereum,
    #[digest(ignore)]
    pub bitcoin_identity: identity::Bitcoin,
    #[digest(prefix = "3001")]
    pub bitcoin_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub bitcoin_amount: asset::Bitcoin,
}

impl From<CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>> for Herc20HbitSwapParams {
    fn from(swap: CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>) -> Self {
        let peer = DialInformation {
            peer_id: swap.peer,
            address_hint: None,
        };

        Self {
            role: swap.role,
            peer,
            ethereum_identity: swap.alpha.identity,
            ethereum_expiry: swap.alpha.absolute_expiry.into(),
            erc20_amount: swap.alpha.amount,
            token_contract: swap.alpha.token_contract,
            bitcoin_identity: swap.beta.identity,
            bitcoin_expiry: swap.beta.absolute_expiry.into(),
            bitcoin_amount: swap.beta.amount,
        }
    }
}

impl ToDigestInput for asset::Bitcoin {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToDigestInput for asset::Ether {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl ToDigestInput for asset::Erc20Quantity {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl ToDigestInput for Timestamp {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().to_bytes().to_vec()
    }
}

impl ToDigestInput for RelativeTime {
    fn to_digest_input(&self) -> Vec<u8> {
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
    pub halight_states: Arc<halight::States>,
    pub db: Sqlite,
}

impl Facade {
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

#[async_trait::async_trait]
impl<T> Save<T> for Facade
where
    Sqlite: Save<T>,
    T: Send + 'static,
{
    async fn save(&self, data: T) -> anyhow::Result<()> {
        self.db.save(data).await
    }
}

#[async_trait::async_trait]
impl<T> InitCommunication<T> for Facade
where
    T: Send + 'static,
    Swarm: InitCommunication<T>,
{
    async fn init_communication(
        &self,
        swap_id: LocalSwapId,
        created_swap: T,
    ) -> anyhow::Result<()> {
        self.swarm.init_communication(swap_id, created_swap).await
    }
}
