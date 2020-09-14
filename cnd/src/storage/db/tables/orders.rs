use crate::storage::{db::schema::orders, NoOrderExists, Text};
use anyhow::{Context, Result};
use comit::{OrderId, Position};
use diesel::{prelude::*, SqliteConnection};
use time::OffsetDateTime;

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "orders"]
pub struct Order {
    pub id: i32,
    #[diesel(deserialize_as = "Text<OrderId>")]
    pub order_id: OrderId,
    #[diesel(deserialize_as = "Text<Position>")]
    pub position: Position,
    pub created_at: i64,
}

impl Order {
    pub fn by_id(conn: &SqliteConnection, id: i32) -> Result<Self> {
        let order = orders::table
            .filter(orders::id.eq(id))
            .first::<Order>(conn)?;

        Ok(order)
    }

    pub fn by_order_id(conn: &SqliteConnection, order_id: OrderId) -> Result<Self> {
        let order = orders::table
            .filter(orders::order_id.eq(Text(order_id)))
            .first::<Order>(conn)
            .with_context(|| NoOrderExists(order_id))?;

        Ok(order)
    }
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "orders"]
pub struct InsertableOrder {
    pub order_id: Text<OrderId>,
    pub position: Text<Position>,
    pub created_at: i64,
}

impl InsertableOrder {
    pub fn new(order_id: OrderId, position: Position, created_at: OffsetDateTime) -> Self {
        Self {
            order_id: Text(order_id),
            position: Text(position),
            created_at: created_at.timestamp(),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<i32> {
        let order_id = self.order_id.0;

        diesel::insert_into(orders::table)
            .values(self)
            .execute(conn)?;

        let order_pk = orders::table
            .filter(orders::order_id.eq(Text(order_id)))
            .select(orders::id)
            .first(conn)?;

        Ok(order_pk)
    }
}
