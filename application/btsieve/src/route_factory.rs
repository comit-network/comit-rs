use crate::{
    query_repository::QueryRepository,
    query_result_repository::{QueryResult, QueryResultRepository},
    routes, web3,
};
use ethereum_support::H256;
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

pub fn create<
    R,
    Q: QueryType + DeserializeOwned + Serialize + Debug + Send + 'static,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
    C: 'static + Send + Sync,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    client: Arc<C>,
    ledger_name: &'static str,
    network: &'static str,
) -> BoxedFilter<(impl Reply,)>
where
    for<'de> R: Deserialize<'de>,
    R: Send + Default + Debug + 'static,
    QueryResult: ToHttpPayload<R, Client = C>,
{
    let route = Q::route();

    let path = warp::path("queries")
        .and(warp::path(ledger_name))
        .and(warp::path(network))
        .and(warp::path(&route));

    let query_repository = warp::any().map(move || Arc::clone(&query_repository));
    let query_result_repository = warp::any().map(move || Arc::clone(&query_result_repository));
    let client = warp::any().map(move || client.clone());

    let create = warp::post2()
        .and(query_repository.clone())
        .and(warp::any().map(move || ledger_name))
        .and(warp::any().map(move || network))
        .and(warp::any().map(move || route))
        .and(warp::body::json())
        .and_then(routes::create_query);

    let retrieve = warp::get2()
        .and(query_repository.clone())
        .and(query_result_repository.clone())
        .and(client.clone())
        .and(warp::path::param::<u32>())
        .and(warp::query::<QueryParams<R>>())
        .and_then(routes::retrieve_query);

    let delete = warp::delete2()
        .and(query_repository)
        .and(query_result_repository)
        .and(warp::path::param::<u32>())
        .and_then(routes::delete_query);

    path.and(create.or(retrieve).or(delete))
        .recover(routes::customize_error)
        .boxed()
}
