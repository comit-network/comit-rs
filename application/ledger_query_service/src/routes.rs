use block_processor::Query;
use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::{QueryResult, QueryResultRepository};
use serde::Serialize;
use std::{env::VarError, sync::Arc};
use warp::{self, Rejection, Reply};

// TODO: Replace warp::Rejection with http-api-problem::HttpApiProblem since it integrates with hyper
// which warp uses under the hood
#[allow(clippy::needless_pass_by_value)]
pub fn end_point_present(endpoint_result: Result<String, VarError>) -> Result<(), Rejection> {
    match endpoint_result {
        Err(_) => Err(warp::reject::not_found()),
        Ok(_) => Ok(()),
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
pub fn handle_new_query<O, Q: Query<O> + Send, QR: QueryRepository<Q>>(
    link_factory: LinkFactory,
    query_repository: Arc<QR>,
    ledger_name: &'static str,
    query_type: &'static str,
    query: Q,
) -> Result<impl Reply, Rejection> {
    let result = query_repository.save(query);

    match result {
        Ok(id) => {
            let uri =
                link_factory.create_link(format!("/queries/{}/{}/{}", ledger_name, query_type, id));
            let reply = warp::reply::with_status(warp::reply(), warp::http::StatusCode::CREATED);
            Ok(warp::reply::with_header(reply, "Location", uri))
        }
        Err(_) => Err(warp::reject::server_error()),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn retrieve_query<
    O,
    Q: Query<O> + Serialize + Send,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    id: u32,
) -> Result<impl Reply, Rejection> {
    let query = query_repository.get(id).ok_or_else(warp::reject);
    match query {
        Ok(query) => {
            let result = query_result_repository.get(id).unwrap_or_default();
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

#[derive(Debug, Serialize, Clone, Default)]
pub struct RetrieveQueryResponse<Q> {
    query: Q,
    matches: QueryResult,
}
