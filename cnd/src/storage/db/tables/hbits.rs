use crate::{
    asset,
    storage::{
        db::{
            schema::*,
            tables::Swap,
            wrapper_types::{Satoshis, U32},
        },
        Text,
    },
};
use anyhow::Result;
use comit::{bitcoin, ledger, Side};
use diesel::{prelude::*, sqlite::SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "hbits"]
pub struct Hbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<ledger::Bitcoin>,
    pub expiry: U32,
    #[diesel(deserialize_as = "Text<bitcoin::Address>")]
    pub final_identity: bitcoin::Address,
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "hbits"]
pub struct InsertableHbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<ledger::Bitcoin>,
    pub expiry: U32,
    // TODO: Rename to make it obvious that this is OUR final address
    pub final_identity: Text<bitcoin::Address>,
    // TODO: Rename to make it obvious that this is the other party's transient identity
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

impl InsertableHbit {
    pub fn new(
        swap_fk: i32,
        asset: asset::Bitcoin,
        network: ledger::Bitcoin,
        expiry: u32,
        final_identity: bitcoin::Address,
        transient_identity: bitcoin::PublicKey,
        side: Side,
    ) -> Self {
        Self {
            swap_id: swap_fk,
            amount: Text(asset.into()),
            network: Text(network),
            expiry: expiry.into(),
            final_identity: Text(final_identity),
            transient_identity: Some(Text(transient_identity)),
            side: Text(side),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(hbits::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl From<Hbit> for asset::Bitcoin {
    fn from(hbit: Hbit) -> Self {
        hbit.amount.0.into()
    }
}
