use diesel::{
    backend::Backend, deserialize, deserialize::FromSql, serialize, serialize::Output, sql_types,
    types::ToSql,
};
use time::OffsetDateTime;

/// Custom diesel new-type that works as long as T implements `Display` and
/// `FromStr`.
#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "sql_types::BigInt"]
pub struct Timestamp(pub OffsetDateTime);

impl<DB> ToSql<sql_types::BigInt, DB> for Timestamp
where
    DB: Backend,
    i64: ToSql<sql_types::BigInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result
    where
        W: std::io::Write,
    {
        self.0.timestamp().to_sql(out)
    }
}

impl<DB> FromSql<sql_types::BigInt, DB> for Timestamp
where
    DB: Backend,
    i64: FromSql<sql_types::BigInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let unix_timestamp = i64::from_sql(bytes)?;

        Ok(Timestamp(OffsetDateTime::from_unix_timestamp(
            unix_timestamp,
        )))
    }
}

impl From<Timestamp> for OffsetDateTime {
    fn from(t: Timestamp) -> Self {
        t.0
    }
}
