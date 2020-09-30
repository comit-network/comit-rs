//! Exposes queries onto the database.
//!
//! In order to hide database details like the `schema.rs` file from the rest of
//! the codebase, we expose queries that compose together diesel's primitives.

use crate::{
    asset,
    storage::{
        db::{schema::*, wrapper_types::Satoshis},
        BtcDaiOrder, NoSwapExists, Order, OrderHbitParams, OrderHerc20Params, ParamsTuple,
        SwapContext, Text,
    },
    LocalSwapId,
};
use anyhow::{Context, Result};
use comit::order::SwapProtocol;
use diesel::{prelude::*, SqliteConnection};
use std::convert::TryFrom;
use time::OffsetDateTime;

pub fn get_swap_context_by_id(conn: &SqliteConnection, id: LocalSwapId) -> Result<SwapContext> {
    let context = swap_contexts::table
        .filter(swap_contexts::id.eq(Text(id)))
        .get_result::<SwapContext>(conn)
        .optional()?
        .ok_or(NoSwapExists(id))?;

    Ok(context)
}

pub fn get_active_swap_contexts(conn: &SqliteConnection) -> Result<Vec<SwapContext>> {
    let query = swaps::table
        .inner_join(swap_contexts::table.on(swap_contexts::id.eq(swaps::local_swap_id)))
        .left_join(completed_swaps::table)
        .filter(completed_swaps::completed_on.is_null())
        .select(swap_contexts::all_columns);

    let contexts = query.load::<SwapContext>(conn)?;

    Ok(contexts)
}

pub fn all_open_btc_dai_orders(conn: &SqliteConnection) -> Result<Vec<(Order, BtcDaiOrder)>> {
    let orders = orders::table
        .inner_join(btc_dai_orders::table)
        .filter(btc_dai_orders::open.ne(Text::<Satoshis>(asset::Bitcoin::ZERO.into())))
        .or_filter(btc_dai_orders::settling.ne(Text::<Satoshis>(asset::Bitcoin::ZERO.into())))
        .load::<(Order, BtcDaiOrder)>(conn)?;

    Ok(orders)
}

pub fn get_orders_to_republish(conn: &SqliteConnection) -> Result<Vec<comit::BtcDaiOrder>> {
    let orders = orders::table
        .inner_join(btc_dai_orders::table)
        .inner_join(order_hbit_params::table)
        .inner_join(order_herc20_params::table)
        .filter(btc_dai_orders::open.ne(Text::<Satoshis>(asset::Bitcoin::ZERO.into())))
        .load::<(Order, BtcDaiOrder, OrderHbitParams, OrderHerc20Params)>(conn)?;

    let orders = orders
        .into_iter()
        .map::<Result<comit::BtcDaiOrder>, _>(
            |(order, btc_dai_order, order_hbit_params, order_herc20_params)| {
                let swap_protocol =
                    SwapProtocol::try_from(ParamsTuple(order_herc20_params, order_hbit_params))
                        .with_context(|| {
                            format!(
                                "failed to construct swap protocol from params for order {}",
                                order.order_id
                            )
                        })?;

                Ok(comit::BtcDaiOrder {
                    id: order.order_id,
                    position: order.position,
                    swap_protocol,
                    created_at: OffsetDateTime::from_unix_timestamp(order.created_at),
                    quantity: btc_dai_order.quantity,
                    price: btc_dai_order.price,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    Ok(orders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        proptest::*,
        storage::{db, InsertableCompletedSwap, Save, Storage},
    };
    use tokio::runtime::Runtime;

    proptest! {
        #[test]
        fn get_active_swap_contexts_does_not_return_completed_swap(
            swap in db::proptest::created_swap(hbit::created_swap(), herc20::created_swap())
        ) {
            let storage = Storage::test();
            let mut runtime = Runtime::new().unwrap();

            let active_swap_contexts = runtime.block_on(async {
                storage.save(swap.clone()).await.unwrap();
                storage.db.do_in_transaction(|conn| {
                    // We only insert one swap, fk is always 1.
                    // The insert would fail if this assumption would not be true thanks to FK-enforcement.
                    InsertableCompletedSwap::new(1, OffsetDateTime::now_utc()).insert(conn)?;
                    get_active_swap_contexts(conn)
                }).await.unwrap()
            });

            assert_eq!(active_swap_contexts, vec![])
        }
    }

    proptest! {
        #[test]
        fn get_active_swap_contexts_returns_not_completed_swap(
            swap in db::proptest::created_swap(hbit::created_swap(), herc20::created_swap())
        ) {
            let storage = Storage::test();
            let mut runtime = Runtime::new().unwrap();

            let active_swap_contexts = runtime.block_on(async {
                storage.save(swap.clone()).await.unwrap();
                storage.db.do_in_transaction(get_active_swap_contexts).await.unwrap()
            });

            assert_eq!(active_swap_contexts.len(), 1)
        }
    }
}
