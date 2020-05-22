use crate::{
    db::{
        self,
        tables::{Halight, Herc20},
        wrapper_types::custom_sql_types::Text,
        Sqlite,
    },
    http_api,
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{halight, herc20, rfc003::DeriveSecret, state::Get, LocalSwapId},
};
use anyhow::Context;
use async_trait::async_trait;
use comit::asset;
use db::tables::{SecretHash, Swap};
use diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl};
use std::sync::Arc;

/// Load data from the storage layer.
#[async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<T>;
}

#[derive(Debug, Clone)]
pub struct Storage {
    db: Sqlite,
    seed: RootSeed,
    herc20_states: Arc<herc20::States>,
    halight_states: Arc<halight::States>,
}

impl Storage {
    pub fn new(
        db: Sqlite,
        seed: RootSeed,
        herc20_states: Arc<herc20::States>,
        halight_states: Arc<halight::States>,
    ) -> Self {
        Self {
            db,
            seed,
            herc20_states,
            halight_states,
        }
    }
}

#[async_trait::async_trait]
impl Load<http_api::Swap<comit::Protocol, comit::Protocol>> for Storage {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::Swap<comit::Protocol, comit::Protocol>> {
        self.db.load_meta_swap(swap_id).await
    }
}

#[async_trait::async_trait]
impl Load<http_api::AliceHerc20HalightBitcoinSwap> for Storage {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::AliceHerc20HalightBitcoinSwap> {
        use crate::db::schema::swaps;

        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.halight_states.get(&swap_id).await?;

        let (halight, herc20) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight: Halight = Halight::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;

                Ok((halight, herc20))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let halight_asset = halight.amount.0.into();

        let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                Ok(http_api::AliceHerc20HalightBitcoinSwap::Finalized {
                    herc20_asset,
                    halight_asset,
                    herc20_refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    herc20_redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    herc20_expiry: herc20.expiry.0.into(),
                    herc20_state: alpha_state,
                    halight_refund_identity: halight
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    halight_redeem_identity: halight
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    halight_state: beta_state,
                    secret,
                })
            }
            _ => Ok(http_api::AliceHerc20HalightBitcoinSwap::Created {
                herc20_asset,
                halight_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl Load<http_api::BobHerc20HalightBitcoinSwap> for Storage {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::BobHerc20HalightBitcoinSwap> {
        use crate::db::schema::swaps;

        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.halight_states.get(&swap_id).await?;

        let (halight, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight: Halight = Halight::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;
                let secret_hash: SecretHash = SecretHash::belonging_to(&swap).first(conn)?;

                Ok((halight, herc20, secret_hash.secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let halight_asset = halight.amount.0.into();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => {
                Ok(http_api::BobHerc20HalightBitcoinSwap::Finalized {
                    herc20_asset,
                    halight_asset,
                    herc20_refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    herc20_redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    herc20_expiry: herc20.expiry.0.into(),
                    herc20_state: alpha_state,
                    halight_refund_identity: halight
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    halight_redeem_identity: halight
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    halight_state: beta_state,
                    secret_hash: secret_hash.0,
                })
            }
            _ => Ok(http_api::BobHerc20HalightBitcoinSwap::Created {
                herc20_asset,
                halight_asset,
            }),
        }
    }
}
