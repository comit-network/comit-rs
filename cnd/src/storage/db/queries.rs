//! Exposes queries onto the database.
//!
//! In order to hide database details like the `schema.rs` file from the rest of
//! the codebase, we expose queries that compose together diesel's primitives.

use crate::{
    asset,
    storage::{
        db::{
            schema::{btc_dai_orders, orders, swap_contexts},
            wrapper_types::Satoshis,
        },
        BtcDaiOrder, NoSwapExists, Order, SwapContext, Text,
    },
    LocalSwapId,
};
use anyhow::Result;
use diesel::{prelude::*, SqliteConnection};

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
