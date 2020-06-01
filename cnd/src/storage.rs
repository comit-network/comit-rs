use crate::{
    db::{
        self,
        tables::{Halight, Herc20},
        wrapper_types::custom_sql_types::Text,
        ForSwap, NoHalightRedeemIdentity, NoHalightRefundIdentity, NoHerc20RedeemIdentity,
        NoHerc20RefundIdentity, NoSecretHash, Save, Sqlite,
    },
    http_api, identity, respawn,
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{halight, herc20, rfc003::DeriveSecret, state::Get},
    LocalSwapId,
};
use anyhow::Context;
use async_trait::async_trait;
use comit::{
    asset,
    network::{WhatAliceLearnedFromBob, WhatBobLearnedFromAlice},
    Protocol, Role,
};
use db::tables::{SecretHash, Swap};
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
impl Load<respawn::Swap<herc20::Params, halight::Params>> for Storage {
    async fn load(
        &self,
        id: LocalSwapId,
    ) -> anyhow::Result<respawn::Swap<herc20::Params, halight::Params>> {
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
            Role::Alice => comit::SecretHash::new(self.seed.derive_swap_seed(id).derive_secret()),
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let swap = respawn::Swap {
            id,
            role,
            alpha: herc20::Params {
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
                    .redeem_identity
                    .ok_or_else(|| NoHerc20RefundIdentity(id))?
                    .0
                    .into(),
                expiry: herc20.expiry.0.into(),
                secret_hash,
            },
            beta: halight::Params {
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
            },
        };

        Ok(swap)
    }
}

#[async_trait::async_trait]
impl LoadAll<respawn::Swap<comit::Protocol, comit::Protocol>> for Storage {
    async fn load_all(
        &self,
    ) -> anyhow::Result<Vec<respawn::Swap<comit::Protocol, comit::Protocol>>> {
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
                    refund_identity: halight.refund_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    redeem_identity: halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?.0,
                    cltv_expiry: halight.cltv_expiry.0.into(),
                    state: alpha_state,
                },
                beta_finalized: http_api::herc20::Finalized {
                    asset: herc20_asset,
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
impl Load<db::Swap> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<db::Swap> {
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

        Ok(db::Swap {
            role: role.0,
            alpha: alpha_protocol.0,
            beta: beta_protocol.0,
        })
    }
}

#[async_trait::async_trait]
impl Load<halight::Params> for Storage {
    async fn load(&self, id: LocalSwapId) -> anyhow::Result<halight::Params> {
        use crate::db::schema::swaps;

        let (swap, halight, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let halight = Halight::belonging_to(&swap).first::<Halight>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, halight, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => comit::SecretHash::new(self.seed.derive_swap_seed(id).derive_secret()),
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let params = halight::Params {
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
        };

        Ok(params)
    }
}

#[async_trait::async_trait]
impl Load<herc20::Params> for Storage {
    async fn load(&self, id: LocalSwapId) -> anyhow::Result<herc20::Params> {
        use crate::db::schema::swaps;

        let (swap, herc20, secret_hash) = self
            .db
            .do_in_transaction::<_, _, anyhow::Error>(move |conn| {
                let key = Text(id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(conn)?;

                let herc20 = Herc20::belonging_to(&swap).first::<Herc20>(conn)?;
                let secret_hash = SecretHash::belonging_to(&swap)
                    .first::<SecretHash>(conn)
                    .optional()?;

                Ok((swap, herc20, secret_hash))
            })
            .await
            .context(db::Error::SwapNotFound)?;

        let role = swap.role.0;
        let secret_hash = match role {
            Role::Alice => comit::SecretHash::new(self.seed.derive_swap_seed(id).derive_secret()),
            Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
        };

        let params = herc20::Params {
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
        };

        Ok(params)
    }
}

// TODO: inserting ethereum redeem and lightning refund from other party is
// common to all redeemers on lightning. Could extract that into a function
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
                self.db.update_halight_refund_identity(
                    conn,
                    local_swap_id,
                    refund_lightning_identity,
                )?;
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;

                Ok(())
            })
            .await
    }
}

// TODO: inserting lightning redeem and ethereum refund from other party is
// common to all redeemers on ethereum. Could extract that into a function
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
