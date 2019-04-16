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
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Debug)]
pub enum Error {
    BitcoinRpcConnection(bitcoin_rpc_client::ClientError),
    BitcoinRpcResponse(bitcoin_rpc_client::RpcError),
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

pub fn create_errored_route(ledger_name: &'static str) -> BoxedFilter<(impl Reply,)> {
    let path = warp::path("queries").and(warp::path(ledger_name));

    let create = warp::post2().and(path).and_then(|| {
        Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: RouteError::NetworkNotFound.into(),
        }))
    });

    let retrieve = warp::get2().and(path).and_then(|| {
        Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: RouteError::NetworkNotFound.into(),
        }))
    });

    let delete = warp::delete2().and(path).and_then(|| {
        Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: RouteError::NetworkNotFound.into(),
        }))
    });

    create
        .or(retrieve)
        .or(delete)
        .recover(routes::customize_error)
        .boxed()
}

pub fn create_endpoints<
    R,
    Q: QueryType + DeserializeOwned + Serialize + Debug + Send + 'static,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
    C: 'static + Send + Sync,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    client: Option<Arc<C>>,
    ledger_name: &'static str,
    registered_network: Option<&'static str>,
) -> BoxedFilter<(impl Reply,)>
where
    for<'de> R: Deserialize<'de>,
    R: Send + Default + Debug + 'static,
    QueryResult: ToHttpPayload<R, Client = C>,
{
    let route = Q::route();

    // create the path
    let path = warp::path("queries");

    let client_option = client.clone();

    // validate ledger function
    let validate_ledger = warp::any().and_then(move || {
        let client_option = client_option.clone();
        client_option.map_or_else(
            || {
                log::error!("Ledger not connected: {:?}", ledger_name);
                Err::<Arc<C>, _>(warp::reject::custom(HttpApiProblemStdError {
                    http_api_problem: RouteError::LedgerNotConnected.into(),
                }))
            },
            |client| Ok(client.clone()),
        )
    });

    // validate network function
    let validate_network =
        warp::path::param::<String>().and_then(move |network| match registered_network {
            Some(registered_network) => {
                if network != registered_network {
                    log::error!("Invalid network passed: {:?}", network);
                    Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
                        http_api_problem: RouteError::NetworkNotFound.into(),
                    }))
                } else {
                    Ok(network)
                }
            }
            None => {
                log::error!("Ledger network not defined {:?}", ledger_name);
                Err::<String, _>(warp::reject::custom(HttpApiProblemStdError {
                    http_api_problem: RouteError::NetworkNotFound.into(),
                }))
            }
        });

    // concat with validators, ledger and network
    let path = path
        .and(warp::path(ledger_name))
        .and(validate_ledger)
        .and(validate_network)
        .and(warp::path(&route));

    let query_repository = warp::any().map(move || Arc::clone(&query_repository));
    let query_result_repository = warp::any().map(move || Arc::clone(&query_result_repository));

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
        .and(warp::path::param::<u32>())
        .and(warp::query::<QueryParams<R>>())
        .and_then(routes::retrieve_query);

    let delete = warp::delete2()
        .and(path)
        .and(query_repository)
        .and(query_result_repository)
        .and(warp::path::param::<u32>())
        .and_then(routes::delete_query);

    create
        .or(retrieve)
        .or(delete)
        .recover(routes::customize_error)
        .boxed()
}
