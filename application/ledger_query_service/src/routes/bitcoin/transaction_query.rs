use bitcoin_support::{
    serialize::BitcoinHash, Address, Block as BitcoinBlock, SpendsTo,
    Transaction as BitcoinTransaction,
};
use block_processor::{Block, Query, Transaction};
use http_api_problem::HttpApiProblem;
use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::{QueryResult, QueryResultRepository};
use rocket::{
    response::{
        status::{Created, NoContent},
        Responder,
    },
    State,
};
use rocket_contrib::Json;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BitcoinTransactionQuery {
    pub to_address: Option<Address>,
    #[serde(default = "default_confirmations")]
    confirmations_needed: u32,
}

fn default_confirmations() -> u32 {
    1
}

#[post(
    "/queries/bitcoin/transactions",
    format = "application/json",
    data = "<query>"
)]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn handle_new_query<'r>(
    query: Json<BitcoinTransactionQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<BitcoinTransactionQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let BitcoinTransactionQuery {
        to_address: None, ..
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/bitcoin/transactions/{}", id)),
        )),
        Err(_) => {
            Err(HttpApiProblem::with_title_from_status(500)
                .set_detail("Failed to create new query"))
        }
    }
}

fn created(url: String) -> Created<Option<()>> {
    Created(url, None)
}

impl Query<BitcoinTransaction> for BitcoinTransactionQuery {
    fn matches(&self, transaction: &BitcoinTransaction) -> bool {
        match self.to_address {
            Some(ref address) => {
                return transaction.spends_to(address);
            }
            None => trace!("to_address not sent, will not used for comparison"),
        }

        false
    }

    fn confirmations_needed(&self) -> u32 {
        self.confirmations_needed
    }
}

impl Transaction for BitcoinTransaction {
    fn transaction_id(&self) -> String {
        self.txid().to_string()
    }
}

impl Block for BitcoinBlock {
    type Transaction = BitcoinTransaction;

    fn blockhash(&self) -> String {
        format!("{:x}", self.header.bitcoin_hash())
    }
    fn prev_blockhash(&self) -> String {
        format!("{:x}", self.header.prev_blockhash)
    }
    fn transactions(&self) -> &[BitcoinTransaction] {
        self.txdata.as_slice()
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RetrieveBitcoinQueryResponse {
    query: BitcoinTransactionQuery,
    matching_transactions: QueryResult,
}

#[get("/queries/bitcoin/transactions/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn retrieve_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<BitcoinTransactionQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<BitcoinTransactionQuery>>>,
) -> Result<Json<RetrieveBitcoinQueryResponse>, HttpApiProblem> {
    let query = query_repository.get(id).ok_or_else(|| {
        HttpApiProblem::with_title_from_status(404).set_detail("The requested query does not exist")
    })?;

    let result = query_result_repository.get(id).unwrap_or_default();

    Ok(Json(RetrieveBitcoinQueryResponse {
        query,
        matching_transactions: result,
    }))
}

#[delete("/queries/bitcoin/transactions/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn delete_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<BitcoinTransactionQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<BitcoinTransactionQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}
