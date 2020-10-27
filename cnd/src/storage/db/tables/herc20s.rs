use crate::storage::{
    db::{
        schema::herc20s,
        tables::Swap,
        wrapper_types::{Erc20Amount, U32},
    },
    Text,
};
use anyhow::Result;
use comit::{asset, ethereum, ethereum::ChainId, Side};
use diesel::{prelude::*, SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "herc20s"]
pub struct Herc20 {
    id: i32,
    swap_id: i32,
    #[diesel(deserialize_as = "Text<Erc20Amount>")]
    pub amount: asset::Erc20Quantity,
    pub chain_id: U32,
    pub expiry: U32,
    pub token_contract: Text<ethereum::Address>,
    pub redeem_identity: Option<Text<ethereum::Address>>,
    pub refund_identity: Option<Text<ethereum::Address>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "herc20s"]
pub struct InsertableHerc20 {
    pub swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub token_contract: Text<ethereum::Address>,
    pub redeem_identity: Option<Text<ethereum::Address>>,
    pub refund_identity: Option<Text<ethereum::Address>>,
    pub side: Text<Side>,
}

impl InsertableHerc20 {
    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(herc20s::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl InsertableHerc20 {
    pub fn new(
        swap_fk: i32,
        asset: asset::Erc20,
        chain_id: ChainId,
        expiry: u32,
        redeem_identity: ethereum::Address,
        refund_identity: ethereum::Address,
        side: Side,
    ) -> Self {
        Self {
            swap_id: swap_fk,
            amount: Text(asset.quantity.into()),
            chain_id: u32::from(chain_id).into(),
            expiry: U32::from(expiry),
            token_contract: Text(asset.token_contract),
            redeem_identity: Some(Text(redeem_identity)),
            refund_identity: Some(Text(refund_identity)),
            side: Text(side),
        }
    }
}

impl From<Herc20> for asset::Erc20 {
    fn from(herc20: Herc20) -> asset::Erc20 {
        asset::Erc20 {
            quantity: herc20.amount,
            token_contract: herc20.token_contract.0,
        }
    }
}
