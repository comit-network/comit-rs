use crate::{
    http_api::routes::rfc003::action::Action,
    swap_protocols::{
        metadata_store,
        rfc003::{self, state_store},
    },
};
use http::StatusCode;
use http_api_problem::HttpApiProblem;
use std::{error::Error, fmt};
use warp::{Rejection, Reply};

#[derive(Debug)]
pub struct HttpApiProblemStdError {
    inner: HttpApiProblem,
}

impl From<HttpApiProblem> for HttpApiProblemStdError {
    fn from(problem: HttpApiProblem) -> Self {
        Self { inner: problem }
    }
}

impl fmt::Display for HttpApiProblemStdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.title)
    }
}

impl Error for HttpApiProblemStdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub fn state_store() -> HttpApiProblem {
    error!("State store didn't have state in it despite having the metadata");
    HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
}
pub fn swap_not_found() -> HttpApiProblem {
    HttpApiProblem::with_title_and_type_from_status(StatusCode::NOT_FOUND)
        .set_title("Swap not found.")
}

pub fn unsupported() -> HttpApiProblem {
    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
        .set_title("Swap not supported.")
        .set_detail("The requested combination of ledgers and assets is not supported.")
}

pub fn deserialize(e: &serde_json::Error) -> HttpApiProblem {
    error!("Failed to deserialize body: {:?}", e);
    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
        .set_title("Invalid body.")
        .set_detail("Failed to deserialize given body.")
}

pub fn serialize(e: serde_json::Error) -> HttpApiProblem {
    error!("Failed to serialize body: {:?}", e);
    HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn not_yet_implemented(feature: &str) -> HttpApiProblem {
    error!("{} not yet implemented", feature);
    HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        .set_title("Feature not yet implemented.")
        .set_detail(format!("{} is not yet implemented! Sorry :(", feature))
}

pub fn action_already_done(action: Action) -> HttpApiProblem {
    error!("{} action has already been done", action);
    HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        .set_title("Action already done.")
}

pub fn invalid_action(action: Action) -> HttpApiProblem {
    error!("{} action is invalid for this swap", action);
    HttpApiProblem::with_title_and_type_from_status(StatusCode::CONFLICT)
        .set_title("Invalid action.")
        .set_detail("Cannot perform requested action for this swap.")
}

pub fn unexpected_query_parameters(action: &str) -> HttpApiProblem {
    error!(
        "Unexpected GET parameters for an {} action type. Expected: None",
        action
    );
    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
        .set_title("Unexpected query parameter(s).")
        .set_detail("This action does not take any query parameters.")
}

impl From<state_store::Error> for HttpApiProblem {
    fn from(e: state_store::Error) -> Self {
        error!("Storage layer failure: {:?}", e);
        HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl From<metadata_store::Error> for HttpApiProblem {
    fn from(e: metadata_store::Error) -> Self {
        error!("Storage layer failure: {:?}", e);
        HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl From<rfc003::state_machine::Error> for HttpApiProblem {
    fn from(e: rfc003::state_machine::Error) -> Self {
        error!("Protocol execution error: {:?}", e);
        HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            .set_title("Protocol execution error.")
    }
}

pub fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = rejection.find_cause::<HttpApiProblemStdError>() {
        let code = err
            .inner
            .status
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let json = warp::reply::json(&err.inner);
        return Ok(warp::reply::with_status(json, code));
    }
    Err(rejection)
}
