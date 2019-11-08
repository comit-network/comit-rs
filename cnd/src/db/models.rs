use crate::{db::schema::swaps, swap_protocols::SwapId};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::{Binary, Integer, Text},
    Insertable, Queryable, *,
};
use ethereum_support::U256;
use std::{convert::TryFrom, fmt, ops::Deref, str::FromStr, string::ToString};

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

#[derive(
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
pub enum Role {
    Alice,
    Bob,
}

#[derive(
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

#[derive(
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
)]
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
    T: fmt::Display + fmt::Debug,
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

/// A suite of tests that ensures the serialization format of the types we use
/// to interact with the database. Changing the format needs to be a conscious
/// activity that involves migration scripts to migrate old data. These tests
/// make sure we don't change the format accidentally!
#[cfg(test)]
mod database_serialization_format_stability_tests {

    use super::*;

    #[test]
    fn swap_id() {
        test::<SwapId>("7f3a105d-ecf2-4cc6-b35c-b4351ac28a34")
    }

    #[test]
    fn ledger_kind() {
        test::<LedgerKind>("Bitcoin");
        test::<LedgerKind>("Ethereum");
        assert_num_variants::<LedgerKind>(2)
    }

    #[test]
    fn asset_kind() {
        test::<AssetKind>("Bitcoin");
        test::<AssetKind>("Ether");
        test::<AssetKind>("Erc20");
        assert_num_variants::<AssetKind>(3)
    }

    #[test]
    fn role() {
        test::<Role>("Alice");
        test::<Role>("Bob");
        assert_num_variants::<Role>(2)
    }

    fn test<T: fmt::Display + FromStr>(stored_value: &str)
    where
        <T as FromStr>::Err: fmt::Debug,
    {
        // First, we verify that we can create T from the given value.
        let read = T::from_str(stored_value).unwrap();

        // Next we convert it to a string again.
        let written = read.to_string();

        // Then if we end up with the same value, our serialization is stable.
        assert_eq!(written, stored_value)
    }

    fn assert_num_variants<E>(expected_number_of_variants: usize)
    where
        E: strum::IntoEnumIterator,
        <E as strum::IntoEnumIterator>::Iterator: Iterator,
    {
        let number_of_variants = E::iter().count();

        assert_eq!(
            number_of_variants,
            expected_number_of_variants,
            "the number of variants for this enum seem to have changed, please add a serialization format test for the new variant and update the expected variant count"
        )
    }
}
