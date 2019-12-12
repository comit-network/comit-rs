use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types,
};
use std::{convert::TryFrom, fmt, ops::Deref, str::FromStr};

/// Custom diesel new-type that works as long as T implements `Display` and
/// `FromStr`.
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "sql_types::Text"]
pub struct Text<T>(pub T);

impl<T> Deref for Text<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<DB, T> ToSql<sql_types::Text, DB> for Text<T>
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
    T: fmt::Display + fmt::Debug,
{
    fn to_sql<W: std::io::Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        let s = self.0.to_string();
        s.to_sql(out)
    }
}

impl<DB, T> FromSql<sql_types::Text, DB> for Text<T>
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        let parsed = T::from_str(s.as_ref())?;

        Ok(Text(parsed))
    }
}

// Custom diesel new type for enforcing storage of a u32
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "sql_types::BigInt"]
pub struct U32(pub u32);

impl<DB> ToSql<sql_types::BigInt, DB> for U32
where
    DB: Backend,
    i64: ToSql<sql_types::BigInt, DB>,
{
    fn to_sql<W: std::io::Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
        let number = i64::try_from(self.0).expect("every u32 fits into a i64");

        number.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::BigInt, DB> for U32
where
    DB: Backend,
    i64: FromSql<sql_types::BigInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let number = i64::from_sql(bytes)?;
        let id = u32::try_from(number)?;

        Ok(U32(id))
    }
}

impl From<U32> for u32 {
    fn from(value: U32) -> u32 {
        value.0
    }
}
