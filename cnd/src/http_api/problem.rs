use crate::{
    db,
    http_api::routes::{
        rfc003::handlers::{post_swap::UnsupportedSwap, InvalidAction, InvalidActionInvocation},
        LndActionError,
    },
};
use http_api_problem::HttpApiProblem;
use warp::{
    http::{self, StatusCode},
    Rejection, Reply,
};

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Missing GET parameters for a {} action type. Expected: {:?}", action, parameters.iter().map(|parameter| parameter.name).collect::<Vec<&str>>())]
pub struct MissingQueryParameters {
    pub action: &'static str,
    pub parameters: &'static [MissingQueryParameter],
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MissingQueryParameter {
    pub name: &'static str,
    pub data_type: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("unexpected GET parameters {parameters:?} for a {action} action type, expected: none")]
pub struct UnexpectedQueryParameters {
    pub action: &'static str,
    pub parameters: &'static [&'static str],
}

// tracing trippers clippy warning, issue reported: https://github.com/tokio-rs/tracing/issues/553
#[allow(clippy::cognitive_complexity)]
pub fn from_anyhow(e: anyhow::Error) -> HttpApiProblem {
    let e = match e.downcast::<HttpApiProblem>() {
        Ok(problem) => return problem,
        Err(e) => e,
    };

    if let Some(db::Error::SwapNotFound) = e.downcast_ref::<db::Error>() {
        return HttpApiProblem::new("Swap not found.").set_status(StatusCode::NOT_FOUND);
    }

    if let Some(e) = e.downcast_ref::<UnexpectedQueryParameters>() {
        tracing::error!("{}", e);

        let mut problem = HttpApiProblem::new("Unexpected query parameter(s).")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This action does not take any query parameters.");

        problem
            .set_value("unexpected_parameters", &e.parameters)
            .expect("parameters will never fail to serialize");

        return problem;
    }

    if let Some(e) = e.downcast_ref::<MissingQueryParameters>() {
        tracing::error!("{}", e);

        let mut problem = HttpApiProblem::new("Missing query parameter(s).")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This action requires additional query parameters.");

        problem
            .set_value("missing_parameters", &e.parameters)
            .expect("parameters will never fail to serialize");

        return problem;
    }

    if e.is::<serde_json::Error>() {
        tracing::error!("deserialization error: {}", e);

        return HttpApiProblem::new("Invalid body.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail(format!("{:?}", e));
    }

    if e.is::<InvalidActionInvocation>() {
        tracing::warn!("{}", e);

        return HttpApiProblem::new("Invalid action invocation")
            .set_status(http::StatusCode::METHOD_NOT_ALLOWED);
    }

    if e.is::<InvalidAction>() {
        tracing::warn!("{}", e);

        return HttpApiProblem::new("Invalid action.")
            .set_status(StatusCode::CONFLICT)
            .set_detail("Cannot perform requested action for this swap.");
    }

    if e.is::<UnsupportedSwap>() {
        tracing::warn!("{}", e);

        return HttpApiProblem::new("Swap not supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("The requested combination of ledgers and assets is not supported.");
    }

    if e.is::<LndActionError>() {
        return HttpApiProblem::new("Action not found.").set_status(StatusCode::NOT_FOUND);
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
