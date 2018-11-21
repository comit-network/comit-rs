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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.title)
    }
}

impl Error for HttpApiProblemStdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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
