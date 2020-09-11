use crate::{
    halbit,
    storage::{
        db::wrapper_types::{Satoshis, U32},
        schema::halbits,
        tables::Swap,
        Insert, IntoInsertable, Sqlite, Text,
    },
};
use comit::{asset, ledger, lightning, Role, Side};
use diesel::{prelude::*, SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "halbits"]
pub struct Halbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<ledger::Bitcoin>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "halbits"]
pub struct InsertableHalbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<ledger::Bitcoin>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

impl From<Halbit> for asset::Bitcoin {
    fn from(halbit: Halbit) -> Self {
        halbit.amount.0.into()
    }
}

impl IntoInsertable for halbit::CreatedSwap {
    type Insertable = InsertableHalbit;

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

        InsertableHalbit {
            swap_id,
            amount: Text(self.asset.into()),
            network: Text(self.network),
            chain: "bitcoin".to_string(), // We currently only support Lightning on top of Bitcoin.
            cltv_expiry: U32(self.cltv_expiry),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

impl Insert<InsertableHalbit> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHalbit,
    ) -> anyhow::Result<()> {
        diesel::insert_into(halbits::table)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}
