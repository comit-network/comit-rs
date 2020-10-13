use crate::storage::{
    db::{schema::order_hbit_params, tables::orders::Order},
    Text,
};
use anyhow::{Context, Result};
use comit::{ledger, Side};
use diesel::{prelude::*, SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "order_hbit_params"]
pub struct OrderHbitParams {
    id: i32,
    pub order_id: i32,
    #[diesel(deserialize_as = "Text<ledger::Bitcoin>")]
    pub network: ledger::Bitcoin,
    #[diesel(deserialize_as = "Text<Side>")]
    pub side: Side,
    #[diesel(deserialize_as = "Text<::bitcoin::Address>")]
    pub our_final_address: ::bitcoin::Address,
    pub expiry_offset: i64,
}

impl OrderHbitParams {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("no hbit params found for order {}", order.order_id))?;

        Ok(params)
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "order_hbit_params"]
pub struct InsertableOrderHbitParams {
    pub order_id: i32,
    pub network: Text<ledger::Bitcoin>,
    pub side: Text<Side>,
    pub our_final_address: Text<::bitcoin::Address>,
    pub expiry_offset: i64,
}

impl InsertableOrderHbitParams {
    pub fn new(
        order_fk: i32,
        network: ledger::Bitcoin,
        our_final_address: ::bitcoin::Address,
        expiry_offset: i64,
        side: Side,
    ) -> Self {
        InsertableOrderHbitParams {
            order_id: order_fk,
            network: Text(network),
            side: Text(side),
            our_final_address: Text(our_final_address),
            expiry_offset,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_hbit_params::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}
