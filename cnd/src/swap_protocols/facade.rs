use super::herc20;
use crate::{
    asset,
    db::{CreatedSwap, Save, Sqlite},
    identity,
    network::{DialInformation, InitCommunication, Swarm},
    storage::{Load, LoadAll, Storage},
    swap_protocols::{hbit, LocalSwapId, Role},
    timestamp::{RelativeTime, Timestamp},
};
use ::comit::network::protocols::announce::SwapDigest;
use digest::Digest;

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
    #[digest(prefix = "2003")]
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
    #[digest(prefix = "3003")]
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
            erc20_amount: swap.beta.asset.quantity,
            token_contract: swap.beta.asset.token_contract,
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
    #[digest(prefix = "2003")]
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
            erc20_amount: swap.alpha.asset.quantity,
            token_contract: swap.alpha.asset.token_contract,
            bitcoin_identity: swap.beta.identity,
            bitcoin_expiry: swap.beta.absolute_expiry.into(),
            bitcoin_amount: swap.beta.amount,
        }
    }
}

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
pub struct Facade {
    pub swarm: Swarm,
    pub db: Sqlite,
    pub storage: Storage,
}

impl Facade {
    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        self.swarm.initiate_communication(id, swap_params).await
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

#[async_trait::async_trait]
impl<T> Load<T> for Facade
where
    Storage: Load<T>,
    T: Send + 'static,
{
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<T> {
        self.storage.load(swap_id).await
    }
}

#[async_trait::async_trait]
impl<T> LoadAll<T> for Facade
where
    Storage: LoadAll<T>,
    T: Send + 'static,
{
    async fn load_all(&self) -> anyhow::Result<Vec<T>> {
        self.storage.load_all().await
    }
}
