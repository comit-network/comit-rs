use block_processor::Query;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use route_factory::{ExpandResult, QueryParams, ShouldExpand};
use serde::Serialize;
use std::sync::Arc;
use url::Url;
use warp::{self, Rejection, Reply};

// TODO: Replace warp::Rejection with http-api-problem::HttpApiProblem since it integrates with hyper
// which warp uses under the hood
#[allow(clippy::needless_pass_by_value)]
pub fn settings_present(settings: Option<()>) -> Result<(), Rejection> {
    match settings {
        None => Err(warp::reject::not_found()),
        Some(_) => Ok(()),
    }
}

pub fn non_empty_query<O, Q: Query<O>>(query: Q) -> Result<Q, Rejection> {
    if query.is_empty() {
        Err(warp::reject())
    } else {
        Ok(query)
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn create_query<O, Q: Query<O> + Send, QR: QueryRepository<Q>>(
    external_url: Url,
    query_repository: Arc<QR>,
    ledger_name: &'static str,
    query_type: &'static str,
    query: Q,
) -> Result<impl Reply, Rejection> {
    let result = query_repository.save(query);

    match result {
        Ok(id) => {
            let uri = external_url
                .join(format!("/queries/{}/{}/{}", ledger_name, query_type, id).as_str())
                .expect("Should be able to join urls")
                .to_string();
            let reply = warp::reply::with_status(warp::reply(), warp::http::StatusCode::CREATED);
            Ok(warp::reply::with_header(reply, "Location", uri))
        }
        Err(_) => Err(warp::reject::server_error()),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn retrieve_query<
    O,
    Q: Query<O> + Serialize + ShouldExpand + Send + ExpandResult,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    client: Option<Arc<<Q as ExpandResult>::Client>>,
    id: u32,
    query_params: QueryParams,
) -> Result<impl Reply, Rejection> {
    let query = query_repository.get(id).ok_or_else(warp::reject);
    match query {
        Ok(query) => {
            let query_result = query_result_repository.get(id).unwrap_or_default();
            let mut result = ResponsePayload::TransactionIds(query_result.0.clone());

            if Q::should_expand(&query_params) {
                match client {
                    Some(client) => match Q::expand_result(&query_result, client) {
                        Ok(data) => {
                            result = ResponsePayload::Transactions(data);
                        }
                        Err(e) => {
                            error!("Could not acquire expanded data: {:?}", e);
                            return Err(warp::reject());
                        }
                    },
                    None => {
                        error!("No Client available to expand data");
                        return Err(warp::reject());
                    }
                }
            }

            Ok(warp::reply::json(&RetrieveQueryResponse {
                query,
                matches: result,
            }))
        }
        Err(e) => Err(e),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn delete_query<
    O,
    Q: Query<O> + Send,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    id: u32,
) -> Result<impl Reply, Rejection> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    Ok(warp::reply::with_status(
        warp::reply(),
        warp::http::StatusCode::NO_CONTENT,
    ))
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum ResponsePayload<T> {
    TransactionIds(Vec<String>),
    Transactions(Vec<T>),
}

impl<T> Default for ResponsePayload<T> {
    fn default() -> Self {
        ResponsePayload::TransactionIds(Vec::new())
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RetrieveQueryResponse<Q, T> {
    query: Q,
    matches: ResponsePayload<T>,
}
