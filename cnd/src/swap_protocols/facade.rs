use super::{herc20, rfc003::DeriveSecret, state::Get};
use crate::{
    asset,
    db::{CreatedSwap, Load, Save, Sqlite},
    http_api, identity,
    network::{DialInformation, InitCommunication, Swarm},
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{halight, hbit, LocalSwapId, Role},
    timestamp::{RelativeTime, Timestamp},
};
use ::comit::network::protocols::announce::SwapDigest;
use digest::Digest;
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
    // We currently only support Han-HALight, therefor 'alpha' is Ethereum and 'beta' is Lightning.
    pub herc20_states: Arc<herc20::States>,
    pub halight_states: Arc<halight::States>,
    pub db: Sqlite,
    pub seed: RootSeed,
}

impl Facade {
    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        self.swarm.initiate_communication(id, swap_params).await
    }

    pub async fn get_created_swap(
        &self,
        id: LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        self.swarm.get_created_swap(id).await
    }

    pub async fn get_alice_herc20_halight_swap(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<Option<http_api::AliceHerc20HalightBitcoinSwap>> {
        let alpha_ledger_state = self.herc20_states.get(&id).await?;
        let beta_ledger_state = self.halight_states.get(&id).await?;

        let (alpha_ledger_state, beta_ledger_state) = match (alpha_ledger_state, beta_ledger_state)
        {
            (Some(alpha_ledger_state), Some(beta_ledger_state)) => {
                (alpha_ledger_state, beta_ledger_state)
            }
            _ => return Ok(None),
        };

        let herc20_params = Load::<herc20::InProgressSwap>::load(self, id).await?;
        let halight_params = Load::<halight::InProgressSwap>::load(self, id).await?;

        let secret = self.seed.derive_swap_seed(id).derive_secret();

        Ok(Some(http_api::AliceHerc20HalightBitcoinSwap {
            alpha_ledger_state,
            beta_ledger_state,
            herc20_params,
            halight_params,
            secret,
        }))
    }

    pub async fn get_bob_herc20_halight_swap(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<Option<http_api::BobHerc20HalightBitcoinSwap>> {
        let alpha_ledger_state = self.herc20_states.get(&id).await?;
        let beta_ledger_state = self.halight_states.get(&id).await?;

        let (alpha_ledger_state, beta_ledger_state) = match (alpha_ledger_state, beta_ledger_state)
        {
            (Some(alpha_ledger_state), Some(beta_ledger_state)) => {
                (alpha_ledger_state, beta_ledger_state)
            }
            _ => return Ok(None),
        };

        let herc20_params = Load::<herc20::InProgressSwap>::load(self, id).await?;
        let halight_params = Load::<halight::InProgressSwap>::load(self, id).await?;

        let secret_hash = self.db.load_secret_hash(id).await?;

        Ok(Some(http_api::BobHerc20HalightBitcoinSwap {
            alpha_ledger_state,
            beta_ledger_state,
            herc20_params,
            halight_params,
            secret_hash,
        }))
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
    Sqlite: Load<T>,
    T: Send + 'static,
{
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<T> {
        self.db.load(swap_id).await
    }
}
