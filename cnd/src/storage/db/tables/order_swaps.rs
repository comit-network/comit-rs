use crate::storage::{
    db::schema::order_swaps,
    tables::{Order, Swap},
};
use anyhow::Result;
use diesel::{prelude::*, sqlite::SqliteConnection};

/// A join table that tracks, which swaps resulted out of which order.
///
/// It is a common join-table naming convention to name these after the two
/// tables that are being associated: In our case, we are associating
/// potentially multiple swaps with a single order, hence the name "OrderSwaps".
#[derive(Associations, Clone, Copy, Debug, Queryable, PartialEq)]
#[belongs_to(Order)]
#[belongs_to(Swap)]
#[table_name = "order_swaps"]
pub struct OrderSwap {
    pub order_id: i32,
    pub swap_id: i32,
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "order_swaps"]
pub struct InsertableOrderSwap {
    pub order_id: i32,
    pub swap_id: i32,
}

impl InsertableOrderSwap {
    pub fn new(swap_pk: i32, order_pk: i32) -> Self {
        Self {
            order_id: order_pk,
            swap_id: swap_pk,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_swaps::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}
