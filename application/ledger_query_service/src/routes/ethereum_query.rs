use ethereum_support::{Address, Bytes, Transaction as EthereumTransaction};
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
use transaction_processor::{Query, Transaction};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EthereumQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
}

#[post("/queries/ethereum", format = "application/json", data = "<query>")]
pub fn handle_new_ethereum_query<'r>(
    query: Json<EthereumQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let EthereumQuery {
        from_address: None,
        to_address: None,
        is_contract_creation: _, // Not enought by itself
        transaction_data: None,
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/ethereum/{}", id)),
        )),
        Err(_) => Err(
            HttpApiProblem::with_title_from_status(500).set_detail("Failed to create new query")
        ),
    }
}

fn created(url: String) -> Created<Option<()>> {
    Created(url, None)
}

impl Query<EthereumTransaction> for EthereumQuery {
    fn matches(&self, transaction: &EthereumTransaction) -> bool {
        let mut result = true;

        if let Some(from_address) = self.from_address {
            trace!("Using from_address for comparison");
            result = result && (transaction.from == from_address);
        }

        if let Some(to_address) = self.to_address {
            trace!("Using to_address for comparison");
            if let Some(tx_to_address) = transaction.to {
                result = result && (tx_to_address == to_address);
            }
        }

        if let Some(is_contract_creation) = self.is_contract_creation {
            trace!("Using is_contract_creation for comparison");
            // to_address is None for contract creations
            result = result && (is_contract_creation == transaction.to.is_none());
        }

        if let Some(ref transaction_data) = self.transaction_data {
            trace!("Using transaction_data for comparison");
            result = result && (transaction.input == *transaction_data);
        }

        result
    }
}

impl Transaction for EthereumTransaction {
    fn transaction_id(&self) -> String {
        self.hash.to_string()
    }
}

#[derive(Serialize, Clone, Default)]
pub struct RetrieveEthereumQueryResponse {
    query: EthereumQuery,
    matching_transactions: QueryResult,
}

#[get("/queries/ethereum/<id>")]
pub fn retrieve_ethereum_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumQuery>>>,
) -> Result<Json<RetrieveEthereumQueryResponse>, HttpApiProblem> {
    let query = query_repository.get(id).ok_or_else(|| {
        HttpApiProblem::with_title_from_status(404).set_detail("The requested query does not exist")
    })?;

    let result = query_result_repository.get(id).unwrap_or_default();

    Ok(Json(RetrieveEthereumQueryResponse {
        query,
        matching_transactions: result,
    }))
}

#[delete("/queries/ethereum/<id>")]
pub fn delete_ethereum_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}
