use crate::{
    identity,
    local_swap_id::LocalSwapId,
    storage::{db::schema, NoSwapExists, Sqlite, Text},
};
use anyhow::Context;
use comit::{Role, Side};
use diesel::{prelude::*, SqliteConnection};

#[macro_export]
macro_rules! swap_id_fk {
    ($local_swap_id:expr) => {
        crate::storage::db::schema::swaps::table
            .filter(crate::storage::db::schema::swaps::local_swap_id.eq(Text($local_swap_id)))
            .select(crate::storage::db::schema::swaps::id)
    };
}

mod btc_dai_orders;
mod halbits;
mod hbits;
mod herc20s;
mod order_hbit_params;
mod order_herc20_params;
mod order_swaps;
mod orders;
mod secret_hashes;
mod swap_contexts;
mod swaps;

pub use btc_dai_orders::{BtcDaiOrder, InsertableBtcDaiOrder};
pub use halbits::{Halbit, InsertableHalbit};
pub use hbits::{Hbit, InsertableHbit};
pub use herc20s::{Herc20, InsertableHerc20};
pub use order_hbit_params::{InsertableOrderHbitParams, OrderHbitParams};
pub use order_herc20_params::{InsertableOrderHerc20Params, OrderHerc20Params};
pub use order_swaps::{InsertableOrderSwap, OrderSwap};
pub use orders::{InsertableOrder, Order};
pub use secret_hashes::{InsertableSecretHash, SecretHash};
pub use swap_contexts::SwapContext;
pub use swaps::{InsertableSwap, Swap};

pub trait IntoInsertable {
    type Insertable;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable;
}

pub trait Insert<I> {
    fn insert(&self, connection: &SqliteConnection, insertable: &I) -> anyhow::Result<()>;
}

trait EnsureSingleRowAffected {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize>;
}

impl EnsureSingleRowAffected for usize {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize> {
        if self != 1 {
            return Err(anyhow::anyhow!(
                "Expected rows to be updated should have been 1 but was {}",
                self
            ));
        }
        Ok(self)
    }
}

/// Load data from tables, A and B are protocol tables.
#[async_trait::async_trait]
pub trait LoadTables<A, B> {
    async fn load_tables(&self, id: LocalSwapId) -> anyhow::Result<Tables<A, B>>;
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
        impl LoadTables<$alpha, $beta> for Sqlite {
            async fn load_tables(&self, id: LocalSwapId) -> anyhow::Result<Tables<$alpha, $beta>> {
                use crate::storage::db::schema::swaps;

                let (swap, alpha, beta, secret_hash) = self
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

impl Sqlite {
    pub fn insert_secret_hash(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        secret_hash: comit::SecretHash,
    ) -> anyhow::Result<()> {
        let swap_id = swap_id_fk!(local_swap_id)
            .first(connection)
            .with_context(|| {
                format!(
                    "failed to find swap_id foreign key for swap {}",
                    local_swap_id
                )
            })?;
        let insertable = InsertableSecretHash::new(swap_id, secret_hash);

        diesel::insert_into(schema::secret_hashes::table)
            .values(insertable)
            .execute(&*connection)
            .with_context(|| format!("failed to insert secret hash for swap {}", local_swap_id))?;

        Ok(())
    }

    pub fn update_halbit_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(schema::halbits::table)
            .filter(schema::halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(schema::halbits::refund_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit refund identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_halbit_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(schema::halbits::table)
            .filter(schema::halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(schema::halbits::redeem_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit redeem identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_herc20_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(schema::herc20s::table)
            .filter(schema::herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(schema::herc20s::refund_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update herc20 refund identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_herc20_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(schema::herc20s::table)
            .filter(schema::herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(schema::herc20s::redeem_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update herc20 redeem identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_hbit_transient_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Bitcoin,
    ) -> anyhow::Result<()> {
        diesel::update(schema::hbits::table)
            .filter(schema::hbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(schema::hbits::transient_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update hbit transient identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    proptest! {
        /// Verify that our database enforces foreign key relations
        ///
        /// We generate a random InsertableHalbit. This comes with a
        /// random swap_id already.
        /// We start with an empty database, so there is no swap that
        /// exists with this swap_id.
        #[test]
        fn fk_relations_are_enforced(
            insertable_halbit in db::tables::insertable_halbit(),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let result = runtime.block_on(db.do_in_transaction(|conn| db.insert(conn, &insertable_halbit)));

            result.unwrap_err();
        }
    }
}
