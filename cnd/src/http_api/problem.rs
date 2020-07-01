use crate::{http_api::ActionNotFound, storage::NoSwapExists};
use http_api_problem::HttpApiProblem;
use warp::{
    http::{self, StatusCode},
    Rejection, Reply,
};

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{ledger:?} is not properly configured, swap involving this ledger are not available.")]
pub struct LedgerNotConfigured {
    pub ledger: &'static str,
}

// tracing triggers clippy warning, issue reported: https://github.com/tokio-rs/tracing/issues/553
#[allow(clippy::cognitive_complexity)]
pub fn from_anyhow(e: anyhow::Error) -> HttpApiProblem {
    let e = match e.downcast::<HttpApiProblem>() {
        Ok(problem) => return problem,
        Err(e) => e,
    };

    if e.is::<NoSwapExists>() {
        tracing::error!("swap was not found");
        return HttpApiProblem::new("Swap not found.").set_status(StatusCode::NOT_FOUND);
    }

    if e.is::<serde_json::Error>() {
        tracing::error!("deserialization error: {}", e);

        return HttpApiProblem::new("Invalid body.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail(format!("{:?}", e));
    }

    if e.is::<ActionNotFound>() {
        return HttpApiProblem::new("Action not found.").set_status(StatusCode::NOT_FOUND);
    }

    if let Some(err) = e.downcast_ref::<LedgerNotConfigured>() {
        tracing::warn!("{}", e);

        return HttpApiProblem::new(format!("{} is not configured.", err.ledger))
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail(format!("{} ledger is not properly configured, swap involving this ledger are not available.", err.ledger));
    }

    tracing::error!("internal error occurred: {:#}", e);

    HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(problem) = rejection.find::<HttpApiProblem>() {
        let code = problem.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let reply = warp::reply::json(problem);
        let reply = warp::reply::with_status(reply, code);
        let reply = warp::reply::with_header(
            reply,
            http::header::CONTENT_TYPE,
            http_api_problem::PROBLEM_JSON_MEDIA_TYPE,
        );

        return Ok(reply);
    }

    Err(rejection)
}
