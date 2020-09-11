use crate::storage::{
    db::{tables::orders::Order, wrapper_types::U32},
    schema::order_herc20_params,
    Text,
};
use anyhow::{Context, Result};
use comit::{ethereum, ethereum::ChainId, Side};
use diesel::{prelude::*, SqliteConnection};

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "order_herc20_params"]
pub struct OrderHerc20Params {
    id: i32,
    pub order_id: i32,
    #[diesel(deserialize_as = "U32")]
    pub chain_id: ChainId,
    #[diesel(deserialize_as = "Text<Side>")]
    pub side: Side,
    #[diesel(deserialize_as = "Text<ethereum::Address>")]
    pub our_htlc_address: ethereum::Address,
    #[diesel(deserialize_as = "Text<ethereum::Address>")]
    pub token_contract: ethereum::Address,
    pub expiry_offset: i64,
}

impl OrderHerc20Params {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("no herc20 params found for order {}", order.order_id))?;

        Ok(params)
    }
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "order_herc20_params"]
pub struct InsertableOrderHerc20Params {
    pub order_id: i32,
    pub chain_id: U32,
    pub side: Text<Side>,
    pub our_htlc_identity: Text<ethereum::Address>,
    pub token_contract: Text<ethereum::Address>,
    pub expiry_offset: i64,
}

impl InsertableOrderHerc20Params {
    pub fn new(
        order_fk: i32,
        chain_id: ChainId,
        our_htlc_identity: ethereum::Address,
        token_contract: ethereum::Address,
        expiry_offset: i64,
        side: Side,
    ) -> Self {
        Self {
            order_id: order_fk,
            chain_id: u32::from(chain_id).into(),
            side: Text(side),
            our_htlc_identity: Text(our_htlc_identity),
            token_contract: Text(token_contract),
            expiry_offset,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_herc20_params::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}
