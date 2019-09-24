use crate::db::schema::swaps;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::{Integer, Text},
    Insertable, Queryable, *,
};
use std::{io::Write, str::FromStr, string::ToString};
use uuid::{parser::ParseError, Uuid};

#[derive(Queryable, Debug, Clone, PartialEq)]
pub struct Swap {
    id: i32,
    pub swap_id: SwapId,
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: Role,
}

#[derive(Insertable, Debug)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub swap_id: SwapId,
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: Role,
}

impl FromStr for SwapId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(SwapId)
    }
}

impl ToString for SwapId {
    fn to_string(&self) -> String {
        self.0.to_hyphenated().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub struct SwapId(Uuid);

impl<DB> ToSql<Text, DB> for SwapId
where
    DB: Backend,
    String: ToSql<Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        self.0.to_hyphenated().to_string().to_sql(out)
    }
}

impl<DB> FromSql<Text, DB> for SwapId
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        let uuid = Uuid::parse_str(&s)?;

        Ok(SwapId(uuid))
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Integer"]
pub enum Role {
    Alice,
    Bob,
}

impl<DB> ToSql<Integer, DB> for Role
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for Role
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(Role::Alice),
            1 => Ok(Role::Bob),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Integer"]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

impl<DB> ToSql<Integer, DB> for LedgerKind
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for LedgerKind
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(LedgerKind::Bitcoin),
            1 => Ok(LedgerKind::Ethereum),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Integer"]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

impl<DB> ToSql<Integer, DB> for AssetKind
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for AssetKind
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(AssetKind::Bitcoin),
            1 => Ok(AssetKind::Ether),
            2 => Ok(AssetKind::Erc20),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}
