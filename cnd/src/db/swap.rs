use crate::{
    db::{rfc003_schema, wrapper_types::custom_sql_types::Text, Error, Sqlite},
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl},
    swap_protocols::rfc003::SwapId,
    Role,
};
use async_trait::async_trait;
use diesel::RunQueryDsl;
use libp2p::{self, PeerId};

/// Retrieve swaps from database.
#[async_trait]
#[ambassador::delegatable_trait]
pub trait Retrieve: Send + Sync + 'static {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Rfc003Swap>;
    async fn all(&self) -> anyhow::Result<Vec<Rfc003Swap>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rfc003Swap {
    pub swap_id: SwapId,
    pub role: Role,
    pub counterparty: PeerId,
}

impl Rfc003Swap {
    pub fn new(swap_id: SwapId, role: Role, counterparty: PeerId) -> Rfc003Swap {
        Rfc003Swap {
            swap_id,
            role,
            counterparty,
        }
    }
}

#[async_trait]
impl Retrieve for Sqlite {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Rfc003Swap> {
        use self::rfc003_schema::rfc003_swaps::dsl::*;

        let record: QueryableSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                rfc003_swaps
                    .filter(swap_id.eq(key))
                    .first(&*connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(Rfc003Swap::from(record))
    }

    async fn all(&self) -> anyhow::Result<Vec<Rfc003Swap>> {
        use self::rfc003_schema::rfc003_swaps::dsl::*;

        let records: Vec<QueryableSwap> = self
            .do_in_transaction(|connection| rfc003_swaps.load(&*connection))
            .await?;

        Ok(records.into_iter().map(|q| q.into()).collect())
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwap {
    pub id: i32,
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
    pub counterparty: Text<PeerId>,
}

impl From<QueryableSwap> for Rfc003Swap {
    fn from(swap: QueryableSwap) -> Rfc003Swap {
        Rfc003Swap {
            swap_id: *swap.swap_id,
            role: *swap.role,
            counterparty: (*swap.counterparty).clone(),
        }
    }
}
