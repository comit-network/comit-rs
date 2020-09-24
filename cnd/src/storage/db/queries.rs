//! Exposes queries onto the database.
//!
//! In order to hide database details like the `schema.rs` file from the rest of
//! the codebase, we expose queries that compose together diesel's primitives.

use crate::{
    asset,
    storage::{
        db::{
            schema::{
                btc_dai_orders, order_hbit_params, order_herc20_params, orders, swap_contexts,
            },
            wrapper_types::Satoshis,
        },
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

pub fn get_all_swap_contexts(conn: &SqliteConnection) -> Result<Vec<SwapContext>> {
    let contexts = swap_contexts::table.load::<SwapContext>(conn)?;

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
