use crate::{
    query_repository::QueryRepository,
    query_result_repository::{QueryResult, QueryResultRepository},
    routes::{self, HttpApiProblemStdError},
    web3,
};
use ethereum_support::H256;
use routes::Error as RouteError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use warp::{self, filters::BoxedFilter, Filter, Rejection, Reply};

// value chosen to accommodate eventual use of 32 byte hashes for id generation
pub const MAX_QUERY_ID_LENGTH: usize = 100;

#[derive(Debug)]
pub enum Error {
    BitcoinRpc(bitcoincore_rpc::Error),
    Web3(web3::Error),
    MissingTransaction(H256),
}

pub trait QueryType {
    fn route() -> &'static str;
}

pub trait ToHttpPayload<R> {
    type Client: 'static + Send + Sync;
    type Item: Serialize + Debug;

    fn to_http_payload(
        &self,
        return_as: &R,
        client: &Self::Client,
    ) -> Result<Vec<Self::Item>, Error>;
}

#[derive(Deserialize, Serialize, Default, Debug, Eq, PartialEq, Hash)]
pub struct QueryParams<R> {
    #[serde(default)]
    pub return_as: R,
}

pub fn create_bitcoin_stub_endpoints(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("queries")
        .and(warp::path("bitcoin"))
        .and(warp::path::param::<String>())
        .and(warp::path(crate::bitcoin::TransactionQuery::route()))
        .and_then(|_| {
            Result::<String, Rejection>::Err(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: RouteError::LedgerNotConnected.into(),
            }))
        })
}

pub fn create_ethereum_stub_endpoints(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let base = warp::path("queries")
        .and(warp::path("ethereum"))
        .and(warp::path::param::<String>());

    let tx_query = base
        .and(warp::path(crate::ethereum::TransactionQuery::route()))
        .and_then(|_| {
            Result::<String, Rejection>::Err(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: RouteError::LedgerNotConnected.into(),
            }))
        });

    let event_query = base
        .and(warp::path(crate::ethereum::EventQuery::route()))
        .and_then(|_| {
            Result::<String, Rejection>::Err(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: RouteError::LedgerNotConnected.into(),
            }))
        });

    tx_query.or(event_query).unify()
}

pub fn create_endpoints<
    R,
    Q: QueryType + DeserializeOwned + Serialize + Debug + Send + Eq + 'static,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
    C: 'static + Send + Sync,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    client: Arc<C>,
    ledger_name: &'static str,
    registered_network: &'static str,
) -> BoxedFilter<(impl Reply,)>
where
    for<'de> R: Deserialize<'de>,
    R: Send + Default + Debug + 'static,
    QueryResult: ToHttpPayload<R, Client = C>,
{
    let route = Q::route();

    // create the path
    let path = warp::path("queries");

    let client = warp::any().map(move || Arc::clone(&client));

    let validate_network = warp::path::param::<String>().and_then(move |network| {
        if network != registered_network {
            log::error!("Invalid network passed: {:?}", network);
            Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: RouteError::NetworkNotFound.into(),
            }))
        } else {
            Ok(network)
        }
    });

    // concat with validators, ledger and network
    let path = path
        .and(warp::path(ledger_name))
        .and(client)
        .and(validate_network)
        .and(warp::path(&route));

    let query_repository = warp::any().map(move || Arc::clone(&query_repository));
    let query_result_repository = warp::any().map(move || Arc::clone(&query_result_repository));

    // validate query id length function
    let validate_query_id_length = warp::path::param::<String>().and_then(|id: String| {
        if id.len() > MAX_QUERY_ID_LENGTH {
            Err(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: RouteError::QueryIdTooLong.into(),
            }))
        } else {
            Ok(id)
        }
    });

    let create = warp::post2()
        .and(path.clone())
        .and(query_repository.clone())
        .and(warp::any().map(move || ledger_name))
        .and(warp::any().map(move || route))
        .and(warp::body::json())
        .and_then(routes::create_query);

    let retrieve = warp::get2()
        .and(path.clone())
        .and(query_repository.clone())
        .and(query_result_repository.clone())
        .and(validate_query_id_length)
        .and(warp::query::<QueryParams<R>>())
        .and_then(routes::retrieve_query);

    let delete = warp::delete2()
        .and(path.clone())
        .and(query_repository.clone())
        .and(query_result_repository)
        .and(warp::path::param::<String>())
        .and_then(routes::delete_query);

    let get_or_create = warp::put2()
        .and(path)
        .and(query_repository)
        .and(validate_query_id_length)
        .and(warp::body::json())
        .and_then(routes::get_or_create_query);

    create.or(retrieve).or(delete).or(get_or_create).boxed()
}
