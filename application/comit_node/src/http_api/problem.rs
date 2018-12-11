use crate::swap_protocols::{metadata_store, rfc003::state_store};
use http::StatusCode;
use http_api_problem::{HttpApiProblem, HttpStatusCode};
use std::{error::Error, fmt};
use warp::{Rejection, Reply};

#[derive(Debug)]
pub struct HttpApiProblemStdError {
    inner: HttpApiProblem,
}

impl HttpApiProblemStdError {
    pub fn new<P: Into<HttpApiProblem>>(inner: P) -> Self {
        Self {
            inner: inner.into(),
        }
    }
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

pub fn serde(_e: &serde_json::Error) -> HttpApiProblem {
    // FIXME: Use error to give more detail to the user
    HttpApiProblem::new("invalid-body")
        .set_status(400)
        .set_detail("Failed to deserialize given body.")
}

pub fn not_yet_implemented(feature: &str) -> HttpApiProblem {
    HttpApiProblem::with_title_and_type_from_status(500)
        .set_detail(format!("{} is not yet implemented! Sorry :(", feature))
}

pub fn action_already_taken() -> HttpApiProblem {
    HttpApiProblem::new("action-already-taken").set_status(400)
}

impl From<state_store::Error> for HttpApiProblem {
    fn from(_e: state_store::Error) -> Self {
        HttpApiProblem::with_title_and_type_from_status(500).set_detail("Storage layer failure")
    }
}

impl From<metadata_store::Error> for HttpApiProblem {
    fn from(_e: metadata_store::Error) -> Self {
        HttpApiProblem::with_title_and_type_from_status(500).set_detail("Storage layer failure")
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
