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
        storage::{db, db::Sqlite},
    };
    use comit::{LockProtocol, Side};
    use tokio::runtime::Runtime;

    // FK given by sqlite deterministically start from 1, we can thus anticipate
    // which FK will be used if we have a fresh test database for every test.
    const FIRST_SWAP_FK: i32 = 1;
    const SECOND_SWAP_FK: i32 = 2;

    proptest! {
        #[test]
        fn get_active_swap_contexts_does_not_return_completed_swap(
            insertable_swap in db::proptest::tables::insertable_swap(),
            insertable_hbit in db::proptest::tables::insertable_hbit(FIRST_SWAP_FK, Side::Alpha),
            insertable_herc20 in db::proptest::tables::insertable_herc20(FIRST_SWAP_FK, Side::Beta),
            insertable_completed_swap in db::proptest::tables::insertable_completed_swap(FIRST_SWAP_FK),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let active_swap_contexts = runtime.block_on(async {
                db.do_in_transaction(|conn| {
                    insertable_swap.insert(conn)?;
                    insertable_hbit.insert(conn)?;
                    insertable_herc20.insert(conn)?;
                    insertable_completed_swap.insert(conn)?;

                    get_active_swap_contexts(conn)
                }).await.unwrap()
            });

            assert_eq!(active_swap_contexts, vec![])
        }
    }

    proptest! {
        #[test]
        fn get_active_swap_contexts_returns_not_completed_swap(
            insertable_swap in db::proptest::tables::insertable_swap(),
            insertable_hbit in db::proptest::tables::insertable_hbit(FIRST_SWAP_FK, Side::Alpha),
            insertable_herc20 in db::proptest::tables::insertable_herc20(FIRST_SWAP_FK, Side::Beta),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let active_swap_contexts = runtime.block_on(async {
                db.do_in_transaction(|conn| {
                    insertable_swap.insert(conn)?;
                    insertable_hbit.insert(conn)?;
                    insertable_herc20.insert(conn)?;

                    get_active_swap_contexts(conn)
                }).await.unwrap()
            });

            assert_eq!(active_swap_contexts.len(), 1)
        }
    }

    proptest! {
        #[test]
        fn get_swap_context_by_id_returns_correct_swap(
            first_insertable_swap in db::proptest::tables::insertable_swap(),
            first_insertable_hbit in db::proptest::tables::insertable_hbit(FIRST_SWAP_FK, Side::Alpha),
            first_insertable_herc20 in db::proptest::tables::insertable_herc20(FIRST_SWAP_FK, Side::Beta),
            second_insertable_swap in db::proptest::tables::insertable_swap(),
            second_insertable_herc20 in db::proptest::tables::insertable_herc20(SECOND_SWAP_FK, Side::Alpha),
            second_insertable_hbit in db::proptest::tables::insertable_hbit(SECOND_SWAP_FK, Side::Beta),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let first_swap_id = first_insertable_swap.local_swap_id;
            let second_swap_id = second_insertable_swap.local_swap_id;

            runtime.block_on(async {
                db.do_in_transaction(|conn| {
                    first_insertable_swap.insert(conn)?;
                    first_insertable_hbit.insert(conn)?;
                    first_insertable_herc20.insert(conn)?;

                    second_insertable_swap.insert(conn)?;
                    second_insertable_hbit.insert(conn)?;
                    second_insertable_herc20.insert(conn)?;

                    Ok(())
                }).await.unwrap();
            });

            let first_swap_context = runtime.block_on(db.do_in_transaction(|conn| get_swap_context_by_id(conn, first_swap_id.0))).unwrap();
            let second_swap_context = runtime.block_on(db.do_in_transaction(|conn| get_swap_context_by_id(conn, second_swap_id.0))).unwrap();

            assert_eq!(first_swap_context.alpha, LockProtocol::Hbit);
            assert_eq!(first_swap_context.beta, LockProtocol::Herc20);

            assert_eq!(second_swap_context.alpha, LockProtocol::Herc20);
            assert_eq!(second_swap_context.beta, LockProtocol::Hbit);
        }
    }
}
