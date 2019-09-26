use crate::db::schema::swaps;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::Text,
    Insertable, Queryable, *,
};
use failure::_core::{
    fmt::{Debug, Display},
    ops::Deref,
};
use std::{str::FromStr, string::ToString};
use uuid::{parser::ParseError, Uuid};

#[derive(Queryable, Debug, Clone, PartialEq)]
pub struct Swap {
    id: i32,
    pub swap_id: SqlText<SwapId>,
    pub alpha_ledger: SqlText<LedgerKind>,
    pub beta_ledger: SqlText<LedgerKind>,
    pub alpha_asset: SqlText<AssetKind>,
    pub beta_asset: SqlText<AssetKind>,
    pub role: SqlText<Role>,
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub swap_id: SqlText<SwapId>,
    pub alpha_ledger: SqlText<LedgerKind>,
    pub beta_ledger: SqlText<LedgerKind>,
    pub alpha_asset: SqlText<AssetKind>,
    pub beta_asset: SqlText<AssetKind>,
    pub role: SqlText<Role>,
}

impl InsertableSwap {
    pub fn new(
        swap_id: SwapId,
        alpha_ledger: LedgerKind,
        beta_ledger: LedgerKind,
        alpha_asset: AssetKind,
        beta_asset: AssetKind,
        role: Role,
    ) -> Self {
        Self {
            swap_id: SqlText(swap_id),
            alpha_ledger: SqlText(alpha_ledger),
            beta_ledger: SqlText(beta_ledger),
            alpha_asset: SqlText(alpha_asset),
            beta_asset: SqlText(beta_asset),
            role: SqlText(role),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub struct SwapId(Uuid);

impl FromStr for SwapId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(SwapId)
    }
}

impl Display for SwapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0.to_hyphenated())
    }
}

#[derive(strum_macros::EnumString, strum_macros::Display, Debug, Clone, Copy, PartialEq)]
pub enum Role {
    Alice,
    Bob,
}

#[derive(strum_macros::EnumString, strum_macros::Display, Debug, Clone, Copy, PartialEq)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

#[derive(strum_macros::EnumString, strum_macros::Display, Debug, Clone, Copy, PartialEq)]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub struct SqlText<T>(pub T);

impl<T> Deref for SqlText<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<DB, T> ToSql<Text, DB> for SqlText<T>
where
    DB: Backend,
    String: ToSql<Text, DB>,
    T: Display + Debug,
{
    fn to_sql<W: std::io::Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        let s = self.0.to_string();
        s.to_sql(out)
    }
}

impl<DB, T> FromSql<Text, DB> for SqlText<T>
where
    DB: Backend,
    String: FromSql<Text, DB>,
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        let parsed = T::from_str(s.as_ref())?;

        Ok(SqlText(parsed))
    }
}
