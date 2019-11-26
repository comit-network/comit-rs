use crate::{
    db::{
        custom_sql_types::Text,
        schema::{self, *},
        Error, Sqlite,
    },
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl},
    swap_protocols::{Role, SwapId},
};
use async_trait::async_trait;
use diesel::RunQueryDsl;
use libp2p::{self, PeerId};

/// Save swap to database.
#[async_trait]
pub trait Save: Send + Sync + 'static {
    async fn save(&self, swap: Swap) -> anyhow::Result<()>;
}

/// Retrieve swaps from database.
#[async_trait]
pub trait Retrieve: Send + Sync + 'static {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Swap>;
    async fn all(&self) -> anyhow::Result<Vec<Swap>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    pub swap_id: SwapId,
    pub role: Role,
    pub counterparty: PeerId,
}

impl Swap {
    pub fn new(swap_id: SwapId, role: Role, counterparty: PeerId) -> Swap {
        Swap {
            swap_id,
            role,
            counterparty,
        }
    }
}

#[async_trait]
impl Save for Sqlite {
    async fn save(&self, swap: Swap) -> anyhow::Result<()> {
        let insertable = InsertableSwap::from(swap);

        self.do_in_transaction(|connection| {
            diesel::insert_into(schema::rfc003_swaps::dsl::rfc003_swaps)
                .values(&insertable)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_swaps"]
struct InsertableSwap {
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
    pub counterparty: Text<PeerId>,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            swap_id: Text(swap.swap_id),
            role: Text(swap.role),
            counterparty: Text(swap.counterparty),
        }
    }
}

#[async_trait]
impl Retrieve for Sqlite {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Swap> {
        use self::schema::rfc003_swaps::dsl::*;

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

        Ok(Swap::from(record))
    }

    async fn all(&self) -> anyhow::Result<Vec<Swap>> {
        use self::schema::rfc003_swaps::dsl::*;

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

impl From<QueryableSwap> for Swap {
    fn from(swap: QueryableSwap) -> Swap {
        Swap {
            swap_id: *swap.swap_id,
            role: *swap.role,
            counterparty: (*swap.counterparty).clone(),
        }
    }
}
