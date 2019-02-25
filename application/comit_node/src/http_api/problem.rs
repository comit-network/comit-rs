use crate::{
    http_api::rfc003::action::Action,
    swap_protocols::{
        metadata_store,
        rfc003::{self, state_store},
    },
};
use http::StatusCode;
use http_api_problem::{HttpApiProblem, HttpStatusCode};
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
    HttpApiProblem::with_title_and_type_from_status(500)
}
pub fn swap_not_found() -> HttpApiProblem {
    HttpApiProblem::new("swap-not-found").set_status(404)
}

pub fn unsupported() -> HttpApiProblem {
    HttpApiProblem::new("swap-not-supported").set_status(400)
}

pub fn deserialize(e: &serde_json::Error) -> HttpApiProblem {
    error!("Failed to deserialize body: {:?}", e);
    HttpApiProblem::new("invalid-body")
        .set_status(400)
        .set_detail("Failed to deserialize given body.")
}

pub fn serialize(e: serde_json::Error) -> HttpApiProblem {
    error!("Failed to serialize body: {:?}", e);
    HttpApiProblem::with_title_and_type_from_status(500)
}

pub fn not_yet_implemented(feature: &str) -> HttpApiProblem {
    error!("{} not yet implemented", feature);
    HttpApiProblem::with_title_and_type_from_status(500)
        .set_detail(format!("{} is not yet implemented! Sorry :(", feature))
}

pub fn action_already_done(action: Action) -> HttpApiProblem {
    error!("{} action has already been done", action);
    HttpApiProblem::new("action-already-done").set_status(409)
}

pub fn invalid_action(action: Action) -> HttpApiProblem {
    error!("{} action is invalid for this swap", action);
    HttpApiProblem::new("invalid-action").set_status(400)
}

impl From<state_store::Error> for HttpApiProblem {
    fn from(e: state_store::Error) -> Self {
        error!("Storage layer failure: {:?}", e);
        HttpApiProblem::with_title_and_type_from_status(500)
    }
}

impl From<metadata_store::Error> for HttpApiProblem {
    fn from(e: metadata_store::Error) -> Self {
        error!("Storage layer failure: {:?}", e);
        HttpApiProblem::with_title_and_type_from_status(500)
    }
}

impl From<rfc003::state_machine::Error> for HttpApiProblem {
    fn from(e: rfc003::state_machine::Error) -> Self {
        error!("Protocol execution error: {:?}", e);
        HttpApiProblem::new("protocol-execution-error").set_status(500)
    }
}

pub fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = rejection.find_cause::<HttpApiProblemStdError>() {
        let code = err
            .inner
            .status
            .unwrap_or(HttpStatusCode::InternalServerError);
        let json = warp::reply::json(&err.inner);
        return Ok(warp::reply::with_status(
            json,
            StatusCode::from_u16(code.to_u16()).unwrap(),
        ));
    }
    Err(rejection)
}
