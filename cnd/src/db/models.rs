use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::{Integer, Text},
};
use ethereum_support::{FromDecimalStr, U256};
use std::{convert::TryFrom, fmt, ops::Deref, str::FromStr, string::ToString};

// Sqlite only supports signed integers, hence we need to wrap this to make it
// type-safe to fetch it from the DB
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Integer"]
pub struct ChainId(pub u32);

impl<DB> ToSql<Integer, DB> for ChainId
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: std::io::Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        let number = i32::try_from(self.0)?;

        number.to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for ChainId
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let number = i32::from_sql(bytes)?;
        let id = u32::try_from(number)?;

        Ok(ChainId(id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, derive_more::FromStr, derive_more::Display)]
pub struct Satoshis(pub u64);

/// The `FromStr` implementation of U256 expects hex but we want to store
/// decimal numbers in the database to aid human-readability.
///
/// This type wraps U256 to provide `FromStr` and `Display` implementations that
/// use decimal numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimalU256(pub U256);

impl FromStr for DecimalU256 {
    type Err = <ethereum_support::U256 as FromDecimalStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U256::from_decimal_str(s).map(DecimalU256)
    }
}

impl fmt::Display for DecimalU256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EthereumAddress(pub ethereum_support::Address);

impl FromStr for EthereumAddress {
    type Err = <ethereum_support::Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(EthereumAddress)
    }
}

impl fmt::Display for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

/// Custom diesel new-type that works as long as T implements `Display` and
/// `FromStr`.
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
    use crate::swap_protocols::{HashFunction, SwapId};

    #[test]
    fn swap_id() {
        test::<SwapId>("7f3a105d-ecf2-4cc6-b35c-b4351ac28a34")
    }

    #[test]
    fn bitcoin_network() {
        test::<bitcoin::Network>("bitcoin");
        test::<bitcoin::Network>("testnet");
        test::<bitcoin::Network>("regtest");
    }

    #[test]
    fn decimal_u256() {
        test::<DecimalU256>("1000000000000000");
    }

    #[test]
    fn bitcoin_amount() {
        test::<Satoshis>("100000000000");
    }

    #[test]
    fn hash_function() {
        test::<HashFunction>("SHA-256");
        assert_num_variants::<HashFunction>(1)
    }

    #[test]
    fn bitcoin_public_key() {
        test::<bitcoin::PublicKey>(
            "0216867374f539badfd90d7b2269008d893ae7bd4f9ee7c695c967d01d6953c401",
        );
    }

    #[test]
    fn ethereum_address() {
        test::<EthereumAddress>("68917b35bacf71dbadf37628b3b7f290f6d88877");
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
