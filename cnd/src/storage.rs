mod db;
mod http_api;
mod seed;

use crate::{
    asset, halbit, hbit, herc20, identity,
    network::{WhatAliceLearnedFromBob, WhatBobLearnedFromAlice},
    spawn, LocalSwapId, LockProtocol, Role, Side,
};
use anyhow::Context;
use async_trait::async_trait;
use diesel::{BelongingToDsl, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use std::sync::Arc;

use chrono::{DateTime, Utc};
pub use db::*;
pub use seed::*;

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

/// Save data to the storage layer.
#[async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, entity: T) -> anyhow::Result<()>;
}

/// Convenience struct to use with `Save` for saving some data T that relates to
/// a LocalSwapId.
#[derive(Debug)]
pub struct ForSwap<T> {
    pub local_swap_id: LocalSwapId,
    pub data: T,
}

/// A facade for the storage layer.
#[derive(Debug, Clone)]
pub struct Storage {
    pub db: Sqlite,
    pub seed: RootSeed,
    pub herc20_states: Arc<herc20::States>,
    pub halbit_states: Arc<halbit::States>,
    pub hbit_states: Arc<hbit::States>,
}

impl Storage {
    pub fn new(db: Sqlite, seed: RootSeed) -> Self {
        Self {
            db,
            seed,
            herc20_states: Arc::new(herc20::States::default()),
            halbit_states: Arc::new(halbit::States::default()),
            hbit_states: Arc::new(hbit::States::default()),
        }
    }

    /// Transient identity used by the hbit HTLC.
    pub fn derive_transient_identity(
        &self,
        swap_id: LocalSwapId,
        role: Role,
        hbit_side: Side,
    ) -> identity::Bitcoin {
        let swap_seed = self.seed.derive_swap_seed(swap_id);
        let sk = match (role, hbit_side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => {
                swap_seed.derive_transient_refund_identity()
            }
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => {
                swap_seed.derive_transient_redeem_identity()
            }
        };

        identity::Bitcoin::from_secret_key(&*crate::SECP, &sk)
    }
}

#[cfg(test)]
impl Storage {
    pub fn test() -> Self {
        Self::new(
            Sqlite::test(),
            RootSeed::new_random(&mut rand::thread_rng()).unwrap(),
        )
    }
}

#[async_trait::async_trait]
impl<A, B, TParamsA, TParamsB> Load<spawn::Swap<TParamsA, TParamsB>> for Storage
where
    Storage: LoadTables<A, B>,
    TParamsA: IntoParams<ProtocolTable = A> + 'static,
    TParamsB: IntoParams<ProtocolTable = B> + 'static,
    A: 'static,
    B: 'static,
{
    async fn load(&self, id: LocalSwapId) -> anyhow::Result<spawn::Swap<TParamsA, TParamsB>> {
        let tab = self.load_tables(id).await?;
        let role = tab.swap.role;
        let secret_hash = derive_or_unwrap_secret_hash(id, self.seed, role, tab.secret_hash)?;

        let alpha = TParamsA::into_params(tab.alpha, id, self.seed, role, secret_hash)?;
        let beta = TParamsB::into_params(tab.beta, id, self.seed, role, secret_hash)?;

        Ok(spawn::Swap {
            role,
            alpha,
            beta,
            start_of_swap: DateTime::<Utc>::from_utc(tab.swap.start_of_swap, Utc),
        })
    }
}

/// Load data from tables, A and B are protocol tables.
#[async_trait::async_trait]
pub trait LoadTables<A, B> {
    async fn load_tables(&self, id: LocalSwapId) -> anyhow::Result<Tables<A, B>>;
}

/// Convert a protocol table, with associated data, into a swap params object.
pub trait IntoParams: Sized {
    type ProtocolTable;

    fn into_params(
        _: Self::ProtocolTable,
        _: LocalSwapId,
        _: RootSeed,
        _: Role,
        _: comit::SecretHash,
    ) -> anyhow::Result<Self>;
}

/// Data required to load in order to construct spawnable swaps (`spawn::Swap`).
#[derive(Debug)]
pub struct Tables<A, B> {
    pub swap: Swap,
    pub alpha: A, // E.g, Herc20
    pub beta: B,  // E.g, Hbit
    pub secret_hash: Option<SecretHash>,
}

macro_rules! impl_load_tables {
    ($alpha:tt, $beta:tt) => {
        #[async_trait::async_trait]
        impl LoadTables<$alpha, $beta> for Storage {
            async fn load_tables(&self, id: LocalSwapId) -> anyhow::Result<Tables<$alpha, $beta>> {
                use self::db::schema::swaps;

                let (swap, alpha, beta, secret_hash) = self
                    .db
                    .do_in_transaction::<_, _>(move |conn| {
                        let key = Text(id);

                        let swap: Swap = swaps::table
                            .filter(swaps::local_swap_id.eq(key))
                            .first(conn)?;

                        let alpha = $alpha::belonging_to(&swap).first::<$alpha>(conn)?;
                        let beta = $beta::belonging_to(&swap).first::<$beta>(conn)?;

                        let secret_hash = SecretHash::belonging_to(&swap)
                            .first::<SecretHash>(conn)
                            .optional()?;

                        Ok((swap, alpha, beta, secret_hash))
                    })
                    .await
                    .context(NoSwapExists(id))?;

                if alpha.side.0 != Side::Alpha {
                    anyhow::bail!(
                        "attempted to load {} as side Alpha but it was {}",
                        stringify!($alpha),
                        alpha.side.0
                    );
                }

                if beta.side.0 != Side::Beta {
                    anyhow::bail!(
                        "attempted to load {} as side Beta but it was {}",
                        stringify!($alpha),
                        beta.side.0
                    );
                }

                Ok(Tables {
                    swap,
                    secret_hash,
                    alpha,
                    beta,
                })
            }
        }
    };
}

impl_load_tables!(Herc20, Halbit);
impl_load_tables!(Halbit, Herc20);
impl_load_tables!(Herc20, Hbit);
impl_load_tables!(Hbit, Herc20);

impl IntoParams for herc20::Params {
    type ProtocolTable = Herc20;

    fn into_params(
        herc20: Self::ProtocolTable,
        id: LocalSwapId,
        _: RootSeed,
        _: Role,
        secret_hash: comit::SecretHash,
    ) -> anyhow::Result<herc20::Params> {
        Ok(herc20::Params {
            asset: asset::Erc20 {
                quantity: herc20.amount.0.into(),
                token_contract: herc20.token_contract.0,
            },
            redeem_identity: herc20
                .redeem_identity
                .ok_or_else(|| NoHerc20RedeemIdentity(id))?
                .0,
            refund_identity: herc20
                .refund_identity
                .ok_or_else(|| NoHerc20RefundIdentity(id))?
                .0,
            expiry: herc20.expiry.0.into(),
            secret_hash,
            chain_id: herc20.chain_id.0.into(),
        })
    }
}

impl IntoParams for halbit::Params {
    type ProtocolTable = Halbit;

    fn into_params(
        halbit: Self::ProtocolTable,
        id: LocalSwapId,
        _: RootSeed,
        _: Role,
        secret_hash: comit::SecretHash,
    ) -> anyhow::Result<halbit::Params> {
        Ok(halbit::Params {
            redeem_identity: halbit
                .redeem_identity
                .ok_or_else(|| NoHalbitRedeemIdentity(id))?
                .0,
            refund_identity: halbit
                .refund_identity
                .ok_or_else(|| NoHalbitRefundIdentity(id))?
                .0,
            cltv_expiry: halbit.cltv_expiry.0.into(),
            asset: halbit.amount.0.into(),
            secret_hash,
        })
    }
}

impl IntoParams for hbit::Params {
    type ProtocolTable = Hbit;

    fn into_params(
        hbit: Self::ProtocolTable,
        id: LocalSwapId,
        seed: RootSeed,
        role: Role,
        secret_hash: comit::SecretHash,
    ) -> anyhow::Result<hbit::Params> {
        let (redeem, refund) = match (hbit.side.0, role) {
            (Side::Alpha, Role::Bob) | (Side::Beta, Role::Alice) => {
                let redeem = identity::Bitcoin::from_secret_key(
                    &*crate::SECP,
                    &seed.derive_swap_seed(id).derive_transient_redeem_identity(),
                );
                let refund = hbit.transient_identity.ok_or(NoHbitRefundIdentity(id))?.0;

                (redeem, refund)
            }
            (Side::Alpha, Role::Alice) | (Side::Beta, Role::Bob) => {
                let redeem = hbit.transient_identity.ok_or(NoHbitRedeemIdentity(id))?.0;
                let refund = identity::Bitcoin::from_secret_key(
                    &*crate::SECP,
                    &seed.derive_swap_seed(id).derive_transient_refund_identity(),
                );

                (redeem, refund)
            }
        };

        Ok(hbit::Params {
            network: hbit.network.0,
            asset: hbit.amount.0.into(),
            redeem_identity: redeem,
            refund_identity: refund,
            expiry: hbit.expiry.0.into(),
            secret_hash,
        })
    }
}

/// Loadable type that provides context for a swap i.e., which protocol
/// is on which side and which role we are playing in the swap.
#[derive(Clone, Copy, Debug)]
pub struct SwapContext {
    pub id: LocalSwapId,
    pub role: Role,
    pub alpha: LockProtocol,
    pub beta: LockProtocol,
}

impl From<tables::SwapContext> for SwapContext {
    fn from(row: tables::SwapContext) -> Self {
        SwapContext {
            id: row.local_swap_id.0,
            role: row.role.0,
            alpha: row.alpha.0,
            beta: row.beta.0,
        }
    }
}

#[async_trait::async_trait]
impl Load<SwapContext> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<SwapContext> {
        use self::schema::swap_contexts;

        let context = self
            .db
            .do_in_transaction(|connection| {
                Ok(swap_contexts::table
                    .filter(swap_contexts::local_swap_id.eq(Text(swap_id)))
                    .get_result::<tables::SwapContext>(connection)
                    .optional()?)
            })
            .await?
            .ok_or(NoSwapExists(swap_id))?;

        Ok(context.into())
    }
}

// Whether or not we get the secret hash from the db or derive it is
// based on which role we are.
fn derive_or_unwrap_secret_hash(
    id: LocalSwapId,
    seed: RootSeed,
    role: Role,
    secret_hash: Option<SecretHash>,
) -> anyhow::Result<comit::SecretHash> {
    let secret_hash = match role {
        Role::Alice => {
            let swap_seed = seed.derive_swap_seed(id);
            comit::SecretHash::new(swap_seed.derive_secret())
        }
        Role::Bob => secret_hash.ok_or_else(|| NoSecretHash(id))?.secret_hash.0,
    };
    Ok(secret_hash)
}

#[async_trait::async_trait]
impl LoadAll<SwapContext> for Storage {
    async fn load_all(&self) -> anyhow::Result<Vec<SwapContext>> {
        use self::schema::swap_contexts;

        let contexts = self
            .db
            .do_in_transaction(|connection| {
                Ok(swap_contexts::table.load::<tables::SwapContext>(connection)?)
            })
            .await?
            .into_iter()
            .map(|context| context.into())
            .collect();

        Ok(contexts)
    }
}

#[async_trait::async_trait]
impl<TCreatedA, TCreatedB, TInsertableA, TInsertableB> Save<CreatedSwap<TCreatedA, TCreatedB>>
    for Storage
where
    TCreatedA: IntoInsertable<Insertable = TInsertableA> + Clone + Send + 'static,
    TCreatedB: IntoInsertable<Insertable = TInsertableB> + Send + 'static,
    TInsertableA: 'static,
    TInsertableB: 'static,
    Sqlite: Insert<TInsertableA> + Insert<TInsertableB>,
{
    async fn save(
        &self,
        CreatedSwap {
            swap_id,
            role,
            peer,
            alpha,
            beta,
            start_of_swap,
            ..
        }: CreatedSwap<TCreatedA, TCreatedB>,
    ) -> anyhow::Result<()> {
        self.db
            .do_in_transaction::<_, _>(move |conn| {
                let swap_fk =
                    InsertableSwap::new(swap_id, peer, role, start_of_swap).insert(conn)?;
                let insertable_alpha = alpha.into_insertable(swap_fk, role, Side::Alpha);
                let insertable_beta = beta.into_insertable(swap_fk, role, Side::Beta);

                self.db.insert(conn, &insertable_alpha)?;
                self.db.insert(conn, &insertable_beta)?;

                Ok(())
            })
            .await?;

        Ok(())
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
        let redeem_ethereum_identity = swap.data.alpha_redeem_identity;
        let refund_lightning_identity = swap.data.beta_refund_identity;

        self.db
            .do_in_transaction(|conn| {
                self.db.update_herc20_redeem_identity(
                    conn,
                    local_swap_id,
                    redeem_ethereum_identity,
                )?;
                self.db.update_halbit_refund_identity(
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
                self.db.update_halbit_redeem_identity(
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
                self.db.update_halbit_redeem_identity(
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
                self.db.update_halbit_refund_identity(
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
        let local_swap_id = swap.local_swap_id;
        let redeem_ethereum_identity = swap.data.alpha_redeem_identity;
        let refund_bitcoin_identity = swap.data.beta_refund_identity;

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
