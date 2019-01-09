use crate::{
    block_processor::Query,
    query_repository::QueryRepository,
    query_result_repository::{QueryResult, QueryResultRepository},
    routes, web3,
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use url::Url;
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Debug)]
pub enum Error {
    InvalidHex,
    BitcoinRpcConnection(bitcoin_rpc_client::ClientError),
    BitcoinRpcResponse(bitcoin_rpc_client::RpcError),
    Web3(web3::Error),
}

#[derive(DebugStub)]
pub struct RouteFactory {
    external_url: Url,
}

pub trait QueryType {
    fn route() -> &'static str;
}

pub trait ExpandResult {
    type Client: 'static + Send + Sync;
    type Item: Serialize;

    fn expand_result(
        result: &QueryResult,
        client: Arc<Self::Client>,
    ) -> Result<Vec<Self::Item>, Error>;
}

pub trait ShouldExpand {
    fn should_expand(query_params: &QueryParams) -> bool;
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct QueryParams {
    #[serde(default)]
    pub expand_results: bool,
}

impl RouteFactory {
    pub fn new(external_url: Url) -> RouteFactory {
        RouteFactory { external_url }
    }

    pub fn create<
        O: 'static,
        Q: Query<O>
            + QueryType
            + ExpandResult
            + ShouldExpand
            + DeserializeOwned
            + Serialize
            + Send
            + 'static,
        QR: QueryRepository<Q>,
        QRR: QueryResultRepository<Q>,
    >(
        &self,
        query_repository: Arc<QR>,
        query_result_repository: Arc<QRR>,
        client: Option<Arc<<Q as ExpandResult>::Client>>,
        ledger_name: &'static str,
    ) -> BoxedFilter<(impl Reply,)> {
        let route = Q::route();

        let path = warp::path("queries")
            .and(warp::path(ledger_name))
            .and(warp::path(&route));

        let external_url = self.external_url.clone();
        let external_url = warp::any().map(move || external_url.clone());
        let query_repository = warp::any().map(move || Arc::clone(&query_repository));
        let query_result_repository = warp::any().map(move || Arc::clone(&query_result_repository));
        let client = warp::any().map(move || client.clone());

        let json_body = warp::body::json().and_then(routes::non_empty_query);

        let create = warp::post2()
            .and(external_url.clone())
            .and(query_repository.clone())
            .and(warp::any().map(move || ledger_name))
            .and(warp::any().map(move || route))
            .and(json_body)
            .and_then(routes::create_query);

        let retrieve = warp::get2()
            .and(query_repository.clone())
            .and(query_result_repository.clone())
            .and(client.clone())
            .and(warp::path::param::<u32>())
            .and(warp::query::<QueryParams>())
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
}
