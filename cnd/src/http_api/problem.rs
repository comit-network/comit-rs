use crate::{
    http_api::ActionNotFound,
    storage::{NoOrderExists, NoSwapExists, NotOpen},
};
use http_api_problem::HttpApiProblem;
use std::error::Error;
use warp::{
    body::BodyDeserializeError,
    http::{self, StatusCode},
    Rejection, Reply,
};

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{ledger:?} is not properly configured, swap involving this ledger are not available.")]
pub struct LedgerNotConfigured {
    pub ledger: &'static str,
}

pub fn from_anyhow(e: anyhow::Error) -> HttpApiProblem {
    // first, check if our inner error is already a problem
    let e = match e.downcast::<HttpApiProblem>() {
        Ok(problem) => return problem,
        Err(e) => e,
    };

    // second, check all errors where we need to downcast to
    if let Some(err) = e.downcast_ref::<LedgerNotConfigured>() {
        return HttpApiProblem::new(format!("{} is not configured.", err.ledger))
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail(format!("{} ledger is not properly configured, swap involving this ledger are not available.", err.ledger));
    }

    let known_error = match &e {
        e if e.is::<NoSwapExists>() => {
            HttpApiProblem::new("Swap not found.").set_status(StatusCode::NOT_FOUND)
        }
        e if e.is::<NoOrderExists>() => {
            HttpApiProblem::new("Order not found.").set_status(StatusCode::NOT_FOUND)
        }
        e if e.is::<NotOpen>() => HttpApiProblem::new("Order can no longer be cancelled.")
            .set_status(StatusCode::BAD_REQUEST),
        e if e.is::<ActionNotFound>() => {
            HttpApiProblem::new("Action not found.").set_status(StatusCode::NOT_FOUND)
        }
        // Use if let here once stable: https://github.com/rust-lang/rust/issues/51114
        e if e.is::<LedgerNotConfigured>() => {
            let e = e
                .downcast_ref::<LedgerNotConfigured>()
                .expect("match arm guard should protect us");

            HttpApiProblem::new(format!("{} is not configured.", e.ledger))
                .set_status(StatusCode::BAD_REQUEST)
                .set_detail(format!("{} ledger is not properly configured, swap involving this ledger are not available.", e.ledger))
        }
        e => {
            tracing::error!("unhandled error: {:#}", e);

            // early return in this branch to avoid double logging the error
            return HttpApiProblem::with_title_and_type_from_status(
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    tracing::info!("route failed because {:#}", e);

    known_error
}

pub async fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(problem) = rejection.find::<HttpApiProblem>() {
        return Ok(problem_to_reply(problem));
    }

    if let Some(invalid_body) = rejection.find::<BodyDeserializeError>() {
        let mut problem = HttpApiProblem::new("Invalid body.").set_status(StatusCode::BAD_REQUEST);

        if let Some(source) = invalid_body.source() {
            problem = problem.set_detail(format!("{}", source));
        }

        return Ok(problem_to_reply(&problem));
    }

    Err(rejection)
}

fn problem_to_reply(problem: &HttpApiProblem) -> impl Reply {
    let code = problem.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let reply = warp::reply::json(problem);
    let reply = warp::reply::with_status(reply, code);

    warp::reply::with_header(
        reply,
        http::header::CONTENT_TYPE,
        http_api_problem::PROBLEM_JSON_MEDIA_TYPE,
    )
}
