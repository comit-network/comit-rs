use crate::{
    herc20,
    storage::{
        db::wrapper_types::{Erc20Amount, U32},
        schema::herc20s,
        tables::Swap,
        Insert, IntoInsertable, Sqlite, Text,
    },
};
use anyhow::Result;
use comit::{asset, ethereum, ethereum::ChainId, Role, Side};
use diesel::{prelude::*, SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "herc20s"]
pub struct Herc20 {
    id: i32,
    swap_id: i32,
    pub amount: Text<Erc20Amount>,
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
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0,
        }
    }
}

impl IntoInsertable for herc20::CreatedSwap {
    type Insertable = InsertableHerc20;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable {
        let redeem_identity = match (role, side) {
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => Some(Text(self.identity)),
            _ => None,
        };
        let refund_identity = match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => Some(Text(self.identity)),
            _ => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHerc20 {
            swap_id,
            amount: Text(self.asset.quantity.into()),
            chain_id: U32(self.chain_id.into()),
            expiry: U32(self.absolute_expiry),
            token_contract: Text(self.asset.token_contract),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

impl Insert<InsertableHerc20> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHerc20,
    ) -> anyhow::Result<()> {
        diesel::insert_into(herc20s::dsl::herc20s)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}
