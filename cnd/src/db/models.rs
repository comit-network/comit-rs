use crate::db::schema::swaps;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::Text,
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

#[derive(Insertable, Debug, Copy, Clone)]
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

macro_rules! impl_to_sql_for_enum {
    ($enum:ident) => {
        impl<DB> ToSql<Text, DB> for $enum
        where
            DB: Backend,
            String: ToSql<Text, DB>,
        {
            fn to_sql<W: Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
                let s = self.to_string();
                s.to_sql(out)
            }
        }
    };
}

#[derive(Display, Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum Role {
    Alice,
    Bob,
}

impl_to_sql_for_enum!(Role);

impl<DB> FromSql<Text, DB> for Role
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;

        match s.as_ref() {
            "Alice" => Ok(Role::Alice),
            "Bob" => Ok(Role::Bob),
            _ => Err(format!("Unknown role: {}", s).into()),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

impl_to_sql_for_enum!(LedgerKind);

impl<DB> FromSql<Text, DB> for LedgerKind
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;

        match s.as_ref() {
            "Bitcoin" => Ok(LedgerKind::Bitcoin),
            "Ethereum" => Ok(LedgerKind::Ethereum),
            _ => Err(format!("Unknown role: {}", s).into()),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

impl_to_sql_for_enum!(AssetKind);

impl<DB> FromSql<Text, DB> for AssetKind
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;

        match s.as_ref() {
            "Bitcoin" => Ok(AssetKind::Bitcoin),
            "Ether" => Ok(AssetKind::Ether),
            "Erc20" => Ok(AssetKind::Erc20),
            _ => Err(format!("Unknown role: {}", s).into()),
        }
    }
}
