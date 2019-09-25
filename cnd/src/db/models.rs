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
}

#[derive(Insertable, Debug)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub swap_id: SwapId,
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
