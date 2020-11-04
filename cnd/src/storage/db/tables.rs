use crate::{
    local_swap_id::LocalSwapId,
    storage::{NoSwapExists, Sqlite, Text},
};
use anyhow::Context;
use comit::{
    expiries::{AlphaOffset, BetaOffset},
    Side,
};
use diesel::prelude::*;

#[macro_export]
macro_rules! swap_id_fk {
    ($local_swap_id:expr) => {
        crate::storage::db::schema::swaps::table
            .filter(crate::storage::db::schema::swaps::local_swap_id.eq(Text($local_swap_id)))
            .select(crate::storage::db::schema::swaps::id)
    };
}

mod btc_dai_orders;
mod completed_swaps;
mod hbits;
mod herc20s;
mod order_hbit_params;
mod order_herc20_params;
mod order_swaps;
mod orders;
mod secret_hashes;
mod swap_contexts;
mod swaps;

use crate::storage::SameSide;
pub use btc_dai_orders::{BtcDaiOrder, InsertableBtcDaiOrder};
use comit::order::SwapProtocol;
pub use completed_swaps::{CompletedSwap, InsertableCompletedSwap};
pub use hbits::{Hbit, InsertableHbit};
pub use herc20s::{Herc20, InsertableHerc20};
pub use order_hbit_params::{InsertableOrderHbitParams, OrderHbitParams};
pub use order_herc20_params::{InsertableOrderHerc20Params, OrderHerc20Params};
pub use order_swaps::{InsertableOrderSwap, OrderSwap};
pub use orders::{InsertableOrder, Order};
pub use secret_hashes::{InsertableSecretHash, SecretHash};
use std::convert::TryFrom;
pub use swap_contexts::SwapContext;
pub use swaps::{InsertableSwap, Swap};
use time::Duration;

/// A newtype for a tuple of params.
///
/// We need this to avoid an overlap with the blanket impl between From and
/// TryFrom. See: https://github.com/rust-lang/rust/issues/50133#issuecomment-646908391
#[derive(Debug)]
pub struct ParamsTuple(pub OrderHerc20Params, pub OrderHbitParams);

impl TryFrom<ParamsTuple> for SwapProtocol {
    type Error = SameSide;

    fn try_from(value: ParamsTuple) -> Result<Self, Self::Error> {
        match value {
            ParamsTuple(
                OrderHerc20Params {
                    side: Side::Beta,
                    expiry_offset: herc20_expiry_offset,
                    ..
                },
                OrderHbitParams {
                    side: Side::Alpha,
                    expiry_offset: hbit_expiry_offset,
                    ..
                },
            ) => Ok(SwapProtocol::HbitHerc20 {
                hbit_expiry_offset: AlphaOffset::from(Duration::seconds(hbit_expiry_offset)),
                herc20_expiry_offset: BetaOffset::from(Duration::seconds(herc20_expiry_offset)),
            }),
            ParamsTuple(
                OrderHerc20Params {
                    side: Side::Alpha,
                    expiry_offset: herc20_expiry_offset,
                    ..
                },
                OrderHbitParams {
                    side: Side::Beta,
                    expiry_offset: hbit_expiry_offset,
                    ..
                },
            ) => Ok(SwapProtocol::Herc20Hbit {
                herc20_expiry_offset: AlphaOffset::from(Duration::seconds(herc20_expiry_offset)),
                hbit_expiry_offset: BetaOffset::from(Duration::seconds(hbit_expiry_offset)),
            }),
            ParamsTuple(
                OrderHerc20Params {
                    side: Side::Alpha, ..
                },
                OrderHbitParams {
                    side: Side::Alpha, ..
                },
            ) => Err(SameSide(Side::Alpha)),
            ParamsTuple(
                OrderHerc20Params {
                    side: Side::Beta, ..
                },
                OrderHbitParams {
                    side: Side::Beta, ..
                },
            ) => Err(SameSide(Side::Beta)),
        }
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

impl_load_tables!(Herc20, Hbit);
impl_load_tables!(Hbit, Herc20);
