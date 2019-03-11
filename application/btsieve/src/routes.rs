use crate::{
    query_repository::QueryRepository,
    query_result_repository::{QueryResult, QueryResultRepository},
    route_factory::{QueryParams, ToHttpPayload},
};
use http_api_problem::{HttpApiProblem, HttpStatusCode};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::{
    error::Error as StdError,
    fmt::{self, Debug},
    sync::Arc,
};
use url::Url;
use warp::{self, Rejection, Reply};

#[derive(Debug)]
pub enum Error {
    QuerySave,
    TransformToPayload,
    QueryNotFound,
}

#[derive(Debug)]
pub struct HttpApiProblemStdError {
    pub http_api_problem: HttpApiProblem,
}

impl fmt::Display for HttpApiProblemStdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.http_api_problem.title)
    }
}

impl StdError for HttpApiProblemStdError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            QuerySave => HttpApiProblem::with_title_and_type_from_status(500)
                .set_detail("Failed to create new query"),
            TransformToPayload => HttpApiProblem::with_title_and_type_from_status(500),
            QueryNotFound => HttpApiProblem::with_title_and_type_from_status(404)
                .set_detail("The requested query does not exist"),
        }
    }
}

pub fn customize_error(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = rejection.find_cause::<HttpApiProblemStdError>() {
        let code = err
            .http_api_problem
            .status
            .unwrap_or(HttpStatusCode::InternalServerError);
        let json = warp::reply::json(&err.http_api_problem);
        return Ok(warp::reply::with_status(
            json,
            StatusCode::from_u16(code.to_u16()).unwrap(),
        ));
    }
    Err(rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn create_query<Q: Send, QR: QueryRepository<Q>>(
    external_url: Url,
    query_repository: Arc<QR>,
    ledger_name: &'static str,
    network: &'static str,
    query_type: &'static str,
    query: Q,
) -> Result<impl Reply, Rejection> {
    let result = query_repository.save(query);

    match result {
        Ok(id) => {
            let uri = external_url
                .join(
                    format!("/queries/{}/{}/{}/{}", ledger_name, network, query_type, id).as_str(),
                )
                .expect("Should be able to join urls")
                .to_string();
            let reply = warp::reply::with_status(warp::reply(), warp::http::StatusCode::CREATED);
            Ok(warp::reply::with_header(reply, "Location", uri))
        }
        Err(_) => Err(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: Error::QuerySave.into(),
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn retrieve_query<
    R: Debug + Default,
    Q: Serialize + Send + Debug,
    QR: QueryRepository<Q>,
    QRR: QueryResultRepository<Q>,
    C: 'static + Send + Sync,
>(
    query_repository: Arc<QR>,
    query_result_repository: Arc<QRR>,
    client: Arc<C>,
    id: u32,
    query_params: QueryParams<R>,
) -> Result<impl Reply, Rejection>
where
    for<'de> R: Deserialize<'de>,
    QueryResult: ToHttpPayload<R, Client = C>,
{
    query_repository
        .get(id)
        .ok_or(Error::QueryNotFound)
        .and_then(|query| {
            query_result_repository
                .get(id)
                .unwrap_or_default()
                .to_http_payload(&query_params.return_as, client.as_ref())
                .map(|matches| RetrieveQueryResponse { query, matches })
                .map(|response| warp::reply::json(&response))
                .map_err(|e| {
                    error!(
                        "failed to transform result for query {} to payload {:?}: {:?}",
                        id, query_params.return_as, e
                    );
                    Error::TransformToPayload
                })
        })
        .map_err(|e| {
            warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: e.into(),
            })
        })
}

#[allow(clippy::needless_pass_by_value)]
pub fn delete_query<Q: Send, QR: QueryRepository<Q>, QRR: QueryResultRepository<Q>>(
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
pub struct RetrieveQueryResponse<Q, T> {
    query: Q,
    matches: T,
}
