//! Exposes commands onto the database.
//!
//! The terminology of commands is borrowed from the CQRS pattern in which
//! queries and commands onto a system are expressed as two distinct concepts.

use crate::{
    asset,
    local_swap_id::LocalSwapId,
    storage::{
        db::{schema::*, wrapper_types::Satoshis},
        BtcDaiOrder, InsertableCompletedSwap, NoOrderForSwap, NotOpen, NotSettling, Order, Text,
    },
};
use anyhow::{Context, Result};
use comit::{OrderId, Quantity};
use diesel::prelude::*;
use time::OffsetDateTime;

/// Move the amount that is settling from open to settling.
///
/// Whilst we don't have partial order matching, this simply means updating
/// `settling` to the amount of `open` and updating `open` to `0`.
///
/// Once we implement partial order matching, this will need to get more
/// sophisticated.
pub fn update_btc_dai_order_to_settling(conn: &SqliteConnection, order_id: OrderId) -> Result<()> {
    let order = Order::by_order_id(conn, order_id)?;
    let btc_dai_order = BtcDaiOrder::by_order(conn, &order)?;

    let affected_rows = diesel::update(&btc_dai_order)
        .set((
            btc_dai_orders::settling.eq(Text::<Satoshis>(btc_dai_order.open.to_inner().into())),
            btc_dai_orders::open.eq(Text::<Satoshis>(asset::Bitcoin::ZERO.into())),
        ))
        .execute(conn)?;

    if affected_rows == 0 {
        anyhow::bail!("failed to mark order {} as settling", order.order_id)
    }

    Ok(())
}

pub fn update_btc_dai_order_to_cancelled(conn: &SqliteConnection, order_id: OrderId) -> Result<()> {
    let order = Order::by_order_id(conn, order_id)?;
    let btc_dai_order = BtcDaiOrder::by_order(conn, &order)?;

    if btc_dai_order.open == Quantity::new(asset::Bitcoin::ZERO) {
        anyhow::bail!(NotOpen(order_id))
    }

    let affected_rows = diesel::update(&btc_dai_order)
        .set((
            btc_dai_orders::cancelled.eq(Text::<Satoshis>(btc_dai_order.open.to_inner().into())),
            btc_dai_orders::open.eq(Text::<Satoshis>(asset::Bitcoin::ZERO.into())),
        ))
        .execute(conn)?;

    if affected_rows == 0 {
        anyhow::bail!("failed to mark order {} as cancelled", order_id)
    }

    Ok(())
}

pub fn update_order_of_swap_to_closed(conn: &SqliteConnection, swap_id: LocalSwapId) -> Result<()> {
    let (order, btc_dai_order) = orders::table
        .inner_join(order_swaps::table.inner_join(swaps::table))
        .inner_join(btc_dai_orders::table)
        .filter(swaps::local_swap_id.eq(Text(swap_id)))
        .select((orders::all_columns, btc_dai_orders::all_columns))
        .first::<(Order, BtcDaiOrder)>(conn)
        .with_context(|| NoOrderForSwap(swap_id))?;

    if btc_dai_order.settling == Quantity::new(asset::Bitcoin::ZERO) {
        anyhow::bail!(NotSettling(order.order_id))
    }

    let affected_rows = diesel::update(&btc_dai_order)
        .set((
            btc_dai_orders::closed.eq(Text::<Satoshis>(btc_dai_order.settling.to_inner().into())),
            btc_dai_orders::settling.eq(Text::<Satoshis>(asset::Bitcoin::ZERO.into())),
        ))
        .execute(conn)?;

    if affected_rows == 0 {
        anyhow::bail!("failed to mark order {} as closed", order.order_id)
    }

    Ok(())
}

pub fn update_order_of_swap_to_failed(conn: &SqliteConnection, swap_id: LocalSwapId) -> Result<()> {
    let (order, btc_dai_order) = orders::table
        .inner_join(order_swaps::table.inner_join(swaps::table))
        .inner_join(btc_dai_orders::table)
        .filter(swaps::local_swap_id.eq(Text(swap_id)))
        .select((orders::all_columns, btc_dai_orders::all_columns))
        .first::<(Order, BtcDaiOrder)>(conn)
        .with_context(|| NoOrderForSwap(swap_id))?;

    if btc_dai_order.settling == Quantity::new(asset::Bitcoin::ZERO) {
        anyhow::bail!(NotSettling(order.order_id))
    }

    let affected_rows = diesel::update(&btc_dai_order)
        .set((
            btc_dai_orders::failed.eq(Text::<Satoshis>(btc_dai_order.settling.to_inner().into())),
            btc_dai_orders::settling.eq(Text::<Satoshis>(asset::Bitcoin::ZERO.into())),
        ))
        .execute(conn)?;

    if affected_rows == 0 {
        anyhow::bail!("failed to mark order {} as failed", order.order_id)
    }

    Ok(())
}

pub fn mark_swap_as_completed(
    conn: &SqliteConnection,
    swap_id: LocalSwapId,
    completed_at: OffsetDateTime,
) -> Result<()> {
    let swap_fk = swap_id_fk!(swap_id).first(conn)?;
    InsertableCompletedSwap::new(swap_fk, completed_at).insert(conn)?;

    Ok(())
}
