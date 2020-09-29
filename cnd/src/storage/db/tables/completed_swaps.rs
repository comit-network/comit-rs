use crate::storage::{
    db::{schema::completed_swaps, Swap},
    Timestamp,
};
use anyhow::Result;
use diesel::{prelude::*, sqlite::SqliteConnection};
use time::OffsetDateTime;

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "completed_swaps"]
pub struct CompletedSwap {
    id: i32,
    pub swap_id: i32,
    #[diesel(deserialize_as = "Timestamp")]
    pub completed_on: OffsetDateTime,
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "completed_swaps"]
pub struct InsertableCompletedSwap {
    pub swap_id: i32,
    pub completed_on: Timestamp,
}

impl InsertableCompletedSwap {
    pub fn new(swap_id: i32, completed_on: OffsetDateTime) -> Self {
        Self {
            swap_id,
            completed_on: Timestamp(completed_on),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(completed_swaps::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}
