use crate::{ethereum, ledger, LocalSwapId};
use comit::{OrderId, Position, Role, Side};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types,
};
use libp2p::PeerId;
use std::{fmt, ops::Deref, str::FromStr};

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
    fn to_sql<W>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result
    where
        W: std::io::Write,
    {
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

macro_rules! impl_from_text {
    ($t:ty) => {
        impl From<Text<$t>> for $t {
            fn from(t: Text<$t>) -> Self {
                t.0
            }
        }
    };
}

impl_from_text!(LocalSwapId);
impl_from_text!(Role);
impl_from_text!(PeerId);
impl_from_text!(ledger::Bitcoin);
impl_from_text!(Side);
impl_from_text!(ethereum::Address);
impl_from_text!(::bitcoin::Address);
impl_from_text!(OrderId);
impl_from_text!(Position);
