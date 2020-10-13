use crate::ethereum;
use comit::{RelativeTime, Timestamp};
use diesel::{
    backend::Backend,
    deserialize, serialize,
    serialize::Output,
    sql_types,
    types::{FromSql, ToSql},
};
use std::convert::TryFrom;

// Custom diesel new type for enforcing storage of a u32
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "sql_types::BigInt"]
pub struct U32(pub u32);

impl<DB> ToSql<sql_types::BigInt, DB> for U32
where
    DB: Backend,
    i64: ToSql<sql_types::BigInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result
    where
        W: std::io::Write,
    {
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

impl From<U32> for ethereum::ChainId {
    fn from(u: U32) -> Self {
        ethereum::ChainId::from(u.0)
    }
}

impl From<U32> for u32 {
    fn from(value: U32) -> u32 {
        value.0
    }
}

impl From<u32> for U32 {
    fn from(value: u32) -> U32 {
        U32(value)
    }
}

impl From<U32> for Timestamp {
    fn from(value: U32) -> Timestamp {
        Timestamp::from(value.0)
    }
}

impl From<U32> for RelativeTime {
    fn from(value: U32) -> RelativeTime {
        RelativeTime::from(value.0)
    }
}
