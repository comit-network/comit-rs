use http::StatusCode;
use http_api_problem::{HttpApiProblem, HttpStatusCode};
use std::{error::Error, fmt};
use swap_protocols::{metadata_store, rfc003::state_store};
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.title)
    }
}

impl Error for HttpApiProblemStdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub fn swap_not_found() -> HttpApiProblem {
    HttpApiProblem::new("swap-not-found").set_status(404)
}

pub fn unsupported() -> HttpApiProblem {
    HttpApiProblem::new("swap-not-supported").set_status(400)
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
    if let Some(ref err) = rejection.find_cause::<HttpApiProblemStdError>() {
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
