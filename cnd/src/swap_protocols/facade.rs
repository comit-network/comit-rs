use super::{herc20, rfc003::DeriveSecret, state::Get};
use crate::{
    asset,
    db::{CreatedSwap, Load, Save, Sqlite},
    http_api,
    http_api::Swap,
    identity,
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

    pub async fn get_alice_herc20_halight_swap(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<http_api::AliceHerc20HalightBitcoinSwap> {
        let alpha_state = self.herc20_states.get(&id).await?;
        let beta_state = self.halight_states.get(&id).await?;

        let (herc20_state, halight_state) = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => (alpha_state, beta_state),
            _ => {
                let swap: Swap<asset::Erc20, asset::Bitcoin> = self.db.load(id).await?;
                return Ok(http_api::AliceHerc20HalightBitcoinSwap::Created {
                    herc20_asset: swap.alpha,
                    halight_asset: swap.beta,
                });
            }
        };

        let (
            herc20_asset,
            herc20::Identities {
                redeem_identity: herc20_redeem_identity,
                refund_identity: herc20_refund_identity,
            },
            herc20_expiry,
        ) = self.load(id).await?;
        let (
            halight_asset,
            halight::Identities {
                redeem_identity: halight_redeem_identity,
                refund_identity: halight_refund_identity,
            },
            cltv_expiry,
        ) = self.load(id).await?;

        let secret = self.seed.derive_swap_seed(id).derive_secret();

        Ok(http_api::AliceHerc20HalightBitcoinSwap::Finalized {
            herc20_asset,
            herc20_refund_identity,
            herc20_redeem_identity,
            herc20_expiry,
            herc20_state,
            halight_asset,
            halight_redeem_identity,
            halight_refund_identity,
            cltv_expiry,
            halight_state,
            secret,
        })
    }

    pub async fn get_bob_herc20_halight_swap(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<http_api::BobHerc20HalightBitcoinSwap> {
        let alpha_state = self.herc20_states.get(&id).await?;
        let beta_state = self.halight_states.get(&id).await?;

        let (herc20_state, halight_state) = match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => (alpha_state, beta_state),
            _ => {
                let swap: Swap<asset::Erc20, asset::Bitcoin> = self.db.load(id).await?;
                return Ok(http_api::BobHerc20HalightBitcoinSwap::Created {
                    herc20_asset: swap.alpha,
                    halight_asset: swap.beta,
                });
            }
        };

        let (
            herc20_asset,
            herc20::Identities {
                redeem_identity: herc20_redeem_identity,
                refund_identity: herc20_refund_identity,
            },
            herc20_expiry,
        ) = self.load(id).await?;
        let (
            halight_asset,
            halight::Identities {
                redeem_identity: halight_redeem_identity,
                refund_identity: halight_refund_identity,
            },
            cltv_expiry,
        ) = self.load(id).await?;

        let secret_hash = self.db.load_secret_hash(id).await?;

        Ok(http_api::BobHerc20HalightBitcoinSwap::Finalized {
            herc20_asset,
            herc20_refund_identity,
            herc20_redeem_identity,
            herc20_expiry,
            herc20_state,
            halight_asset,
            halight_redeem_identity,
            halight_refund_identity,
            cltv_expiry,
            halight_state,
            secret_hash,
        })
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
