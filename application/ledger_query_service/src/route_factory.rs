use bitcoin_rpc_client;
use bitcoin_support;
use block_processor::Query;
use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::{QueryResult, QueryResultRepository};
use routes;
use serde::{de::DeserializeOwned, Serialize};
use std::{env::VarError, sync::Arc};
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Debug)]
pub enum Error {
    TransactionIdConversionFailure(bitcoin_support::hash::HexError),
    BitcoinRpcConnectionFailure(bitcoin_rpc_client::ClientError),
    BitcoinRpcResponseFailure(bitcoin_rpc_client::RpcError),
}

#[derive(DebugStub)]
pub struct RouteFactory {
    link_factory: LinkFactory,
}

pub trait QueryType {
    fn route() -> &'static str;
}

pub trait ExpandData {
    type Client: 'static + Send + Sync;
    type Item: Serialize;

    fn expand_data(
        result: &QueryResult,
        client: Arc<Self::Client>,
    ) -> Result<Vec<Self::Item>, Error>;
}

pub trait MustExpand {
    fn must_expand(query_params: &QueryParams) -> bool;
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct QueryParams {
    pub inline_transactions: Option<bool>,
}

impl RouteFactory {
    pub fn new(link_factory: LinkFactory) -> RouteFactory {
        RouteFactory { link_factory }
    }

    pub fn create<
        O: 'static,
        Q: Query<O>
            + QueryType
            + ExpandData
            + MustExpand
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
        client: Arc<<Q as ExpandData>::Client>,
        endpoint: Result<String, VarError>,
        ledger_name: &'static str,
    ) -> BoxedFilter<(impl Reply,)> {
        let endpoint = warp::any()
            .map(move || endpoint.clone())
            .and_then(routes::end_point_present);

        let route = Q::route();

        let path = warp::path("queries")
            .and(warp::path(ledger_name))
            .and(warp::path(&route));

        let link_factory = self.link_factory.clone();
        let link_factory = warp::any().map(move || link_factory.clone());
        let query_repository = warp::any().map(move || query_repository.clone());
        let query_result_repository = warp::any().map(move || query_result_repository.clone());
        let client = warp::any().map(move || client.clone());

        let json_body = warp::body::json().and_then(routes::non_empty_query);

        let create = warp::post2()
            .and(link_factory.clone())
            .and(query_repository.clone())
            .and(warp::any().map(move || ledger_name))
            .and(warp::any().map(move || route))
            .and(json_body)
            .and_then(routes::create_query);

        let retrieve = warp::get2()
            .and(query_repository.clone())
            .and(query_result_repository.clone())
            .and(client.clone())
            .and(warp::path::param())
            .and(warp::query::<QueryParams>())
            .and_then(routes::retrieve_query);

        let delete = warp::delete2()
            .and(query_repository)
            .and(query_result_repository)
            .and(warp::path::param())
            .and_then(routes::delete_query);

        endpoint
            .and(path)
            .and(create.or(retrieve).or(delete))
            .map(|_, reply| reply)
            .boxed()
    }
}
