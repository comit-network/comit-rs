use http::StatusCode;
use http_api_problem::HttpApiProblem;
use std::{error::Error as StdError, fmt};
use warp::{Filter, Rejection, Reply};

const HEADER_NAME: &str = "Expected-Version";

#[derive(Debug)]
enum Error {
    MissingExpectedVersionHeader,
    VersionMismatch {
        expected_version: String,
        actual_version: String,
    },
}

#[derive(Debug)]
struct HttpApiProblemStdError {
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
            MissingExpectedVersionHeader => {
                HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                    .set_detail(format!("Missing {} header", HEADER_NAME))
            }
            VersionMismatch {
                expected_version,
                actual_version,
            } => HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                .set_detail(format!(
                    "Expected version: {}. Actual version: {}.",
                    expected_version, actual_version
                )),
        }
    }
}

pub fn customize_error(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = rejection.find_cause::<HttpApiProblemStdError>() {
        let code = err
            .http_api_problem
            .status
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let json = warp::reply::json(&err.http_api_problem);
        return Ok(warp::reply::with_status(json, code));
    }
    Err(rejection)
}

pub fn validate() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::<String>(HEADER_NAME)
        .or_else(move |_| {
            Err(warp::reject::custom(HttpApiProblemStdError {
                http_api_problem: Error::MissingExpectedVersionHeader.into(),
            }))
        })
        .and_then(|value: String| {
            let expected_version = value;
            let actual_version = env!("CARGO_PKG_VERSION").to_string();

            if expected_version == actual_version {
                Ok(())
            } else {
                Err(warp::reject::custom(HttpApiProblemStdError {
                    http_api_problem: Error::VersionMismatch {
                        expected_version,
                        actual_version,
                    }
                    .into(),
                }))
            }
        })
        .untuple_one()
}
