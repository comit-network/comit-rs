use crate::{
    asset,
    db::{
        self,
        tables::{Halight, Hbit, Herc20, SecretHash, Swap},
        wrapper_types::custom_sql_types::Text,
        ForSwap, NoHalightRedeemIdentity, NoHalightRefundIdentity, NoHerc20RedeemIdentity,
        NoHerc20RefundIdentity, NoSecretHash, Save, Sqlite,
    },
    halight, hbit, herc20, http_api, identity,
    network::{WhatAliceLearnedFromBob, WhatBobLearnedFromAlice},
    seed::RootSeed,
    start_swap,
    swap_protocols::state::Get,
    DecisionSwap, LocalSwapId, Protocol, Role, Side,
};
use anyhow::Context;
use async_trait::async_trait;
use bitcoin::Network;
use diesel::{
    sql_types, BelongingToDsl, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
};
use std::sync::Arc;

/// Load data for a particular swap from the storage layer.
#[async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<T>;
}

/// Load all data of type T from the storage layer.
#[async_trait]
pub trait LoadAll<T>: Send + Sync + 'static {
    async fn load_all(&self) -> anyhow::Result<Vec<T>>;
}

#[derive(Debug, Clone)]
pub struct Storage {
    db: Sqlite,
    seed: RootSeed,
    herc20_states: Arc<herc20::States>,
    halight_states: Arc<halight::States>,
    hbit_states: Arc<hbit::States>,
}

impl Storage {
    pub fn new(
        db: Sqlite,
        seed: RootSeed,
        herc20_states: Arc<herc20::States>,
        halight_states: Arc<halight::States>,
        hbit_states: Arc<hbit::States>,
    ) -> Self {
        Self {
            db,
            seed,
            herc20_states,
            halight_states,
            hbit_states,
        }
    }
}

#[async_trait::async_trait]
impl Load<http_api::DecisionSwap> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<http_api::DecisionSwap> {
        self.db.load_meta_swap(swap_id).await
    }
}

#[async_trait::async_trait]
impl Load<start_swap::Swap<herc20::Params, halight::Params>> for Storage {
    async fn load(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<start_swap::Swap<herc20::Params, halight::Params>> {
        use crate::db::schema::swaps;

        let (swap, halight, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight = Halight::belonging_to(&swap).first::<Halight>(conn)?;
                let herc20 = Herc20::belonging_to(&swap).first::<Herc20>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, halight, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => {
                let swap_seed = self.seed.derive_swap_seed(id);
                comit::SecretHash::new(swap_seed.derive_secret())
            }
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let swap = start_swap::Swap {
            role,
            alpha: build_herc20_params(herc20, secret_hash, id)?,
            beta: build_halight_params(halight, secret_hash, id)?,
            start_of_swap: swap.start_of_swap,
        };

        Ok(swap)
    }
}

#[async_trait::async_trait]
impl Load<start_swap::Swap<halight::Params, herc20::Params>> for Storage {
    async fn load(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<start_swap::Swap<halight::Params, herc20::Params>> {
        use crate::db::schema::swaps;

        let (swap, halight, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight = Halight::belonging_to(&swap).first::<Halight>(conn)?;
                let herc20 = Herc20::belonging_to(&swap).first::<Herc20>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, halight, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => {
                let swap_seed = self.seed.derive_swap_seed(id);
                comit::SecretHash::new(swap_seed.derive_secret())
            }
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let swap = start_swap::Swap {
            role,
            alpha: build_halight_params(halight, secret_hash, id)?,
            beta: build_herc20_params(herc20, secret_hash, id)?,
            start_of_swap: swap.start_of_swap,
        };

        Ok(swap)
    }
}

#[async_trait::async_trait]
impl Load<start_swap::Swap<herc20::Params, hbit::Params>> for Storage {
    async fn load(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<start_swap::Swap<herc20::Params, hbit::Params>> {
        use crate::db::schema::swaps;

        let (swap, hbit, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let hbit = Hbit::belonging_to(&swap).first::<Hbit>(conn)?;
                let herc20 = Herc20::belonging_to(&swap).first::<Herc20>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, hbit, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => {
                let swap_seed = self.seed.derive_swap_seed(id);
                comit::SecretHash::new(swap_seed.derive_secret())
            }
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let swap = start_swap::Swap {
            role,
            alpha: build_herc20_params(herc20, secret_hash, id)?,
            beta: build_hbit_params(self.seed, hbit, id, role, secret_hash)?,
            start_of_swap: swap.start_of_swap,
        };

        Ok(swap)
    }
}

#[async_trait::async_trait]
impl Load<start_swap::Swap<hbit::Params, herc20::Params>> for Storage {
    async fn load(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<start_swap::Swap<hbit::Params, herc20::Params>> {
        use crate::db::schema::swaps;

        let (swap, hbit, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let hbit = Hbit::belonging_to(&swap).first::<Hbit>(conn)?;
                let herc20 = Herc20::belonging_to(&swap).first::<Herc20>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, hbit, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => {
                let swap_seed = self.seed.derive_swap_seed(id);
                comit::SecretHash::new(swap_seed.derive_secret())
            }
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let swap = start_swap::Swap {
            role,
            alpha: build_hbit_params(self.seed, hbit, id, role, secret_hash)?,
            beta: build_herc20_params(herc20, secret_hash, id)?,
            start_of_swap: swap.start_of_swap,
        };

        Ok(swap)
    }
}

#[async_trait::async_trait]
impl LoadAll<DecisionSwap> for Storage {
    async fn load_all(&self) -> anyhow::Result<Vec<DecisionSwap>> {
        self.db.load_all_respawn_meta_swaps().await
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::AliceSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::halight::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::AliceSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::halight::Finalized,
        >,
    > {
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
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                http_api::herc20::Finalized,
                http_api::halight::Finalized,
            >::Finalized {
                alpha_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::halight::Finalized {
                    asset: halight_asset,
                    network: halight.network.0.into(),
                    refund_identity: halight.refund_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    redeem_identity: halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    state: beta_state,
                },
                secret,
            }),
            _ => Ok(http_api::AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                http_api::herc20::Finalized,
                http_api::halight::Finalized,
            >::Created {
                alpha_created: herc20_asset,
                beta_created: halight_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::AliceSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::halight::Finalized,
            http_api::herc20::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::AliceSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::halight::Finalized,
            http_api::herc20::Finalized,
        >,
    > {
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
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::AliceSwap::<
                asset::Bitcoin,
                asset::Erc20,
                http_api::halight::Finalized,
                http_api::herc20::Finalized,
            >::Finalized {
                beta_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: alpha_state,
                },
                alpha_finalized: http_api::halight::Finalized {
                    asset: halight_asset,
                    network: halight.network.0.into(),
                    refund_identity: halight.refund_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    redeem_identity: halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    state: beta_state,
                },
                secret,
            }),
            _ => Ok(http_api::AliceSwap::<
                asset::Bitcoin,
                asset::Erc20,
                http_api::halight::Finalized,
                http_api::herc20::Finalized,
            >::Created {
                beta_created: herc20_asset,
                alpha_created: halight_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::BobSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::halight::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::BobSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::halight::Finalized,
        >,
    > {
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
                let secret_hash: Option<SecretHash> =
                    SecretHash::belonging_to(&swap).first(conn).optional()?;

                Ok((halight, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let halight_asset = halight.amount.0.into();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::BobSwap::<
                asset::Erc20,
                asset::Bitcoin,
                http_api::herc20::Finalized,
                http_api::halight::Finalized,
            >::Finalized {
                alpha_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::halight::Finalized {
                    asset: halight_asset,
                    network: halight.network.0.into(),
                    refund_identity: halight.refund_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    redeem_identity: halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    state: beta_state,
                },
                secret_hash: secret_hash
                    .ok_or(db::Error::SecretHashNotSet)?
                    .secret_hash
                    .0,
            }),
            _ => Ok(http_api::BobSwap::<
                asset::Erc20,
                asset::Bitcoin,
                http_api::herc20::Finalized,
                http_api::halight::Finalized,
            >::Created {
                alpha_created: herc20_asset,
                beta_created: halight_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::BobSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::halight::Finalized,
            http_api::herc20::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::BobSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::halight::Finalized,
            http_api::herc20::Finalized,
        >,
    > {
        use crate::db::schema::swaps;

        let alpha_state = self.halight_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let (halight, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight: Halight = Halight::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;
                let secret_hash: Option<SecretHash> =
                    SecretHash::belonging_to(&swap).first(conn).optional()?;

                Ok((halight, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let halight_asset = halight.amount.0.into();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::BobSwap::<
                asset::Bitcoin,
                asset::Erc20,
                http_api::halight::Finalized,
                http_api::herc20::Finalized,
            >::Finalized {
                alpha_finalized: http_api::halight::Finalized {
                    asset: halight_asset,
                    network: halight.network.0.into(),
                    refund_identity: halight.refund_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    redeem_identity: halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: beta_state,
                },
                secret_hash: secret_hash
                    .ok_or(db::Error::SecretHashNotSet)?
                    .secret_hash
                    .0,
            }),
            _ => Ok(http_api::BobSwap::<
                asset::Bitcoin,
                asset::Erc20,
                http_api::halight::Finalized,
                http_api::herc20::Finalized,
            >::Created {
                alpha_created: halight_asset,
                beta_created: herc20_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::AliceSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::hbit::FinalizedAsRedeemer,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::AliceSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::hbit::FinalizedAsRedeemer,
        >,
    > {
        use crate::db::schema::swaps;

        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.hbit_states.get(&swap_id).await?;

        let (herc20, hbit) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;
                let hbit: Hbit = Hbit::belonging_to(&swap).first(conn)?;

                Ok((herc20, hbit))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let hbit_asset = hbit.amount.0.into();

        let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::AliceSwap::Finalized {
                alpha_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::hbit::FinalizedAsRedeemer {
                    asset: hbit_asset,
                    network: hbit.network.0.into(),
                    transient_refund_identity: hbit
                        .transient_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    transient_redeem_identity: self
                        .seed
                        .derive_swap_seed(swap_id)
                        .derive_transient_redeem_identity(),
                    final_redeem_identity: hbit.final_identity.0,
                    expiry: hbit.expiry.0.into(),
                    state: beta_state,
                },
                secret,
            }),
            _ => Ok(http_api::AliceSwap::Created {
                alpha_created: herc20_asset,
                beta_created: hbit_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::BobSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::hbit::FinalizedAsFunder,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::BobSwap<
            asset::Erc20,
            asset::Bitcoin,
            http_api::herc20::Finalized,
            http_api::hbit::FinalizedAsFunder,
        >,
    > {
        use crate::db::schema::swaps;

        let alpha_state = self.herc20_states.get(&swap_id).await?;
        let beta_state = self.hbit_states.get(&swap_id).await?;

        let (herc20, hbit, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let hbit: Hbit = Hbit::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;
                let secret_hash: Option<SecretHash> =
                    SecretHash::belonging_to(&swap).first(conn).optional()?;

                Ok((herc20, hbit, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };
        let hbit_asset = hbit.amount.0.into();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::BobSwap::Finalized {
                alpha_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::hbit::FinalizedAsFunder {
                    asset: hbit_asset,
                    network: hbit.network.0.into(),
                    transient_redeem_identity: hbit
                        .transient_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    transient_refund_identity: self
                        .seed
                        .derive_swap_seed(swap_id)
                        .derive_transient_refund_identity(),
                    final_refund_identity: hbit.final_identity.0,
                    expiry: hbit.expiry.0.into(),
                    state: beta_state,
                },
                secret_hash: secret_hash
                    .ok_or(db::Error::SecretHashNotSet)?
                    .secret_hash
                    .0,
            }),
            _ => Ok(http_api::BobSwap::Created {
                alpha_created: herc20_asset,
                beta_created: hbit_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::AliceSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::hbit::FinalizedAsFunder,
            http_api::herc20::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::AliceSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::hbit::FinalizedAsFunder,
            http_api::herc20::Finalized,
        >,
    > {
        use crate::db::schema::swaps;

        let alpha_state = self.hbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let (hbit, herc20) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let hbit: Hbit = Hbit::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;

                Ok((hbit, herc20))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let hbit_asset = hbit.amount.0.into();
        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };

        let secret = self.seed.derive_swap_seed(swap_id).derive_secret();

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::AliceSwap::Finalized {
                alpha_finalized: http_api::hbit::FinalizedAsFunder {
                    asset: hbit_asset,
                    network: hbit.network.0.into(),
                    transient_refund_identity: self
                        .seed
                        .derive_swap_seed(swap_id)
                        .derive_transient_refund_identity(),
                    transient_redeem_identity: hbit
                        .transient_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    final_refund_identity: hbit.final_identity.0,
                    expiry: hbit.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: beta_state,
                },
                secret,
            }),
            _ => Ok(http_api::AliceSwap::Created {
                alpha_created: hbit_asset,
                beta_created: herc20_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl
    Load<
        http_api::BobSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::hbit::FinalizedAsRedeemer,
            http_api::herc20::Finalized,
        >,
    > for Storage
{
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<
        http_api::BobSwap<
            asset::Bitcoin,
            asset::Erc20,
            http_api::hbit::FinalizedAsRedeemer,
            http_api::herc20::Finalized,
        >,
    > {
        use crate::db::schema::swaps;

        let alpha_state = self.hbit_states.get(&swap_id).await?;
        let beta_state = self.herc20_states.get(&swap_id).await?;

        let (hbit, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let hbit: Hbit = Hbit::belonging_to(&swap).first(conn)?;
                let herc20: Herc20 = Herc20::belonging_to(&swap).first(conn)?;
                let secret_hash: Option<SecretHash> =
                    SecretHash::belonging_to(&swap).first(conn).optional()?;

                Ok((hbit, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let hbit_asset = hbit.amount.0.into();
        let herc20_asset = asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        };

        match (alpha_state, beta_state) {
            (Some(alpha_state), Some(beta_state)) => Ok(http_api::BobSwap::Finalized {
                alpha_finalized: http_api::hbit::FinalizedAsRedeemer {
                    asset: hbit_asset,
                    network: hbit.network.0.into(),
                    transient_redeem_identity: self
                        .seed
                        .derive_swap_seed(swap_id)
                        .derive_transient_redeem_identity(),
                    transient_refund_identity: hbit
                        .transient_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0,
                    final_redeem_identity: hbit.final_identity.0,
                    expiry: hbit.expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
                    chain_id: herc20.chain_id.0.into(),
                    refund_identity: herc20
                        .refund_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    redeem_identity: herc20
                        .redeem_identity
                        .ok_or(db::Error::IdentityNotSet)?
                        .0
                        .into(),
                    expiry: herc20.expiry.0.into(),
                    state: beta_state,
                },
                secret_hash: secret_hash
                    .ok_or(db::Error::SecretHashNotSet)?
                    .secret_hash
                    .0,
            }),
            _ => Ok(http_api::BobSwap::Created {
                alpha_created: hbit_asset,
                beta_created: herc20_asset,
            }),
        }
    }
}

#[async_trait::async_trait]
impl Load<DecisionSwap> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<DecisionSwap> {
        #[derive(QueryableByName)]
        struct Result {
            #[sql_type = "sql_types::Text"]
            role: Text<Role>,
            #[sql_type = "sql_types::Text"]
            alpha_protocol: Text<Protocol>,
            #[sql_type = "sql_types::Text"]
            beta_protocol: Text<Protocol>,
        }

        let Result { role, alpha_protocol, beta_protocol } = self.db.do_in_transaction(|connection| {
            // Here is how this works:
            // - COALESCE selects the first non-null value from a list of values
            // - We use 3 sub-selects to select a static value (i.e. 'halight', etc) if that particular child table has a row with a foreign key to the parent table
            // - We do this two times, once where we limit the results to rows that have `ledger` set to `Alpha` and once where `ledger` is set to `Beta`
            //
            // The result is a view with 3 columns: `role`, `alpha_protocol` and `beta_protocol` where the `*_protocol` columns have one of the values `halight`, `herc20` or `hbit`
            diesel::sql_query(
                r#"
                SELECT
                    role,
                    COALESCE(
                       (SELECT 'halight' from halights where halights.swap_id = swaps.id and halights.side = 'Alpha'),
                       (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Alpha'),
                       (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Alpha')
                    ) as alpha_protocol,
                    COALESCE(
                       (SELECT 'halight' from halights where halights.swap_id = swaps.id and halights.side = 'Beta'),
                       (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Beta'),
                       (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Beta')
                    ) as beta_protocol
                from swaps
                    where local_swap_id = ?
            "#,
            )
                .bind::<sql_types::Text, _>(Text(swap_id))
                .get_result(connection)
        }).await.context(db::Error::SwapNotFound)?;

        Ok(DecisionSwap {
            id: swap_id,
            role: role.0,
            alpha: alpha_protocol.0,
            beta: beta_protocol.0,
        })
    }
}

#[async_trait::async_trait]
impl Load<identity::Bitcoin> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<identity::Bitcoin> {
        let swap: http_api::DecisionSwap = self.load(swap_id).await?;

        let sk = match swap {
            http_api::DecisionSwap {
                role: Role::Alice,
                alpha: Protocol::Hbit,
                ..
            }
            | http_api::DecisionSwap {
                role: Role::Bob,
                beta: Protocol::Hbit,
                ..
            } => self
                .seed
                .derive_swap_seed(swap_id)
                .derive_transient_refund_identity(),
            http_api::DecisionSwap {
                role: Role::Alice,
                beta: Protocol::Hbit,
                ..
            }
            | http_api::DecisionSwap {
                role: Role::Bob,
                alpha: Protocol::Hbit,
                ..
            } => self
                .seed
                .derive_swap_seed(swap_id)
                .derive_transient_redeem_identity(),
            _ => anyhow::bail!(HbitNotInvolved(swap_id)),
        };

        Ok(identity::Bitcoin::from_secret_key(&*crate::SECP, &sk))
    }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("could not derive Bitcoin identity for swap not involving hbit: {0}")]
pub struct HbitNotInvolved(pub LocalSwapId);

#[async_trait::async_trait]
impl Save<ForSwap<WhatAliceLearnedFromBob<identity::Ethereum, identity::Lightning>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatAliceLearnedFromBob<identity::Ethereum, identity::Lightning>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let refund_lightning_identity = swap.data.beta_refund_identity;
        let redeem_ethereum_identity = swap.data.alpha_redeem_identity;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;
                self.db.update_halight_refund_identity(
                    conn,
                    local_swap_id,
                    refund_lightning_identity,
                )?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatBobLearnedFromAlice<identity::Ethereum, identity::Lightning>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatBobLearnedFromAlice<identity::Ethereum, identity::Lightning>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let redeem_lightning_identity = swap.data.beta_redeem_identity;
        let refund_ethereum_identity = swap.data.alpha_refund_identity;
        let secret_hash = swap.data.secret_hash;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_halight_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_lightning_identity,
                )?;
                self.db.update_herc20_refund_identity(
                    conn,
                    local_swap_id,
                    refund_ethereum_identity,
                )?;
                self.db
                    .insert_secret_hash(conn, local_swap_id, secret_hash)?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatAliceLearnedFromBob<identity::Lightning, identity::Ethereum>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatAliceLearnedFromBob<identity::Lightning, identity::Ethereum>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let redeem_lightning_identity = swap.data.alpha_redeem_identity;
        let refund_ethereum_identity = swap.data.beta_refund_identity;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_halight_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_lightning_identity,
                )?;
                self.db.update_herc20_refund_identity(
                    conn,
                    local_swap_id,
                    refund_ethereum_identity,
                )?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatBobLearnedFromAlice<identity::Lightning, identity::Ethereum>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatBobLearnedFromAlice<identity::Lightning, identity::Ethereum>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let redeem_ethereum_identity = swap.data.beta_redeem_identity;
        let refund_lightning_identity = swap.data.alpha_refund_identity;
        let secret_hash = swap.data.secret_hash;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;
                self.db.update_halight_refund_identity(
                    conn,
                    local_swap_id,
                    refund_lightning_identity,
                )?;
                self.db
                    .insert_secret_hash(conn, local_swap_id, secret_hash)?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatAliceLearnedFromBob<identity::Ethereum, identity::Bitcoin>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatAliceLearnedFromBob<identity::Ethereum, identity::Bitcoin>>,
    ) -> anyhow::Result<()> {
        // identity is the transient one in here

        let local_swap_id = swap.local_swap_id;
        let refund_bitcoin_identity = swap.data.beta_refund_identity;
        let redeem_ethereum_identity = swap.data.alpha_redeem_identity;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;
                self.db.update_hbit_transient_identity(
                    conn,
                    local_swap_id,
                    refund_bitcoin_identity,
                )?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatBobLearnedFromAlice<identity::Ethereum, identity::Bitcoin>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatBobLearnedFromAlice<identity::Ethereum, identity::Bitcoin>>,
    ) -> anyhow::Result<()> {
        // identity is the transient one in here

        let local_swap_id = swap.local_swap_id;
        let redeem_bitcoin_identity = swap.data.beta_redeem_identity;
        let refund_ethereum_identity = swap.data.alpha_refund_identity;
        let secret_hash = swap.data.secret_hash;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_hbit_transient_identity(
                    conn,
                    local_swap_id,
                    redeem_bitcoin_identity,
                )?;
                self.db.update_herc20_refund_identity(
                    conn,
                    local_swap_id,
                    refund_ethereum_identity,
                )?;
                self.db
                    .insert_secret_hash(conn, local_swap_id, secret_hash)?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatAliceLearnedFromBob<identity::Bitcoin, identity::Ethereum>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatAliceLearnedFromBob<identity::Bitcoin, identity::Ethereum>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let transient_redeem_bitcoin_identity = swap.data.alpha_redeem_identity;
        let refund_ethereum_identity = swap.data.beta_refund_identity;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_hbit_transient_identity(
                    conn,
                    local_swap_id,
                    transient_redeem_bitcoin_identity,
                )?;
                self.db.update_herc20_refund_identity(
                    conn,
                    local_swap_id,
                    refund_ethereum_identity,
                )?;

                Ok(())
            })
            .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<WhatBobLearnedFromAlice<identity::Bitcoin, identity::Ethereum>>> for Storage {
    async fn save(
        &self,
        swap: ForSwap<WhatBobLearnedFromAlice<identity::Bitcoin, identity::Ethereum>>,
    ) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let transient_refund_bitcoin_identity = swap.data.alpha_refund_identity;
        let redeem_ethereum_identity = swap.data.beta_redeem_identity;
        let secret_hash = swap.data.secret_hash;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_hbit_transient_identity(
                    conn,
                    local_swap_id,
                    transient_refund_bitcoin_identity,
                )?;
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;
                self.db
                    .insert_secret_hash(conn, local_swap_id, secret_hash)?;

                Ok(())
            })
            .await
    }
}

fn build_hbit_params(
    seed: RootSeed,
    hbit: Hbit,
    id: LocalSwapId,
    role: Role,
    secret_hash: comit::SecretHash,
) -> anyhow::Result<hbit::Params> {
    let (redeem, refund) = match (hbit.side.0, role) {
        (Side::Alpha, Role::Bob) | (Side::Beta, Role::Alice) => {
            let redeem = comit::bitcoin::PublicKey::from_secret_key(
                &*crate::SECP,
                &seed.derive_swap_seed(id).derive_transient_redeem_identity(),
            );
            let refund = hbit.transient_identity.ok_or(db::Error::IdentityNotSet)?.0;

            (redeem, refund)
        }
        (Side::Alpha, Role::Alice) | (Side::Beta, Role::Bob) => {
            let redeem = hbit.transient_identity.ok_or(db::Error::IdentityNotSet)?.0;
            let refund = comit::bitcoin::PublicKey::from_secret_key(
                &*crate::SECP,
                &seed.derive_swap_seed(id).derive_transient_refund_identity(),
            );

            (redeem, refund)
        }
    };

    Ok(hbit::Params {
        network: Network::Regtest,
        asset: hbit.amount.0.into(),
        redeem_identity: redeem,
        refund_identity: refund,
        expiry: hbit.expiry.0.into(),
        secret_hash,
    })
}

fn build_halight_params(
    halight: Halight,
    secret_hash: comit::SecretHash,
    id: LocalSwapId,
) -> anyhow::Result<halight::Params> {
    Ok(halight::Params {
        redeem_identity: halight
            .redeem_identity
            .ok_or_else(|| NoHalightRedeemIdentity(id))?
            .0,
        refund_identity: halight
            .refund_identity
            .ok_or_else(|| NoHalightRefundIdentity(id))?
            .0,
        cltv_expiry: halight.cltv_expiry.0.into(),
        asset: halight.amount.0.into(),
        secret_hash,
    })
}

fn build_herc20_params(
    herc20: Herc20,
    secret_hash: comit::SecretHash,
    id: LocalSwapId,
) -> anyhow::Result<herc20::Params> {
    Ok(herc20::Params {
        asset: asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0.into(),
        },
        redeem_identity: herc20
            .redeem_identity
            .ok_or_else(|| NoHerc20RedeemIdentity(id))?
            .0
            .into(),
        refund_identity: herc20
            .refund_identity
            .ok_or_else(|| NoHerc20RefundIdentity(id))?
            .0
            .into(),
        expiry: herc20.expiry.0.into(),
        secret_hash,
    })
}
