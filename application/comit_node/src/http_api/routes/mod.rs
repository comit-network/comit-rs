use crate::http_api::HttpApiProblemStdError;
use http_api_problem::HttpApiProblem;
use warp::Rejection;

pub mod index;
pub mod peers;
pub mod rfc003;

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
	warp::reject::custom(HttpApiProblemStdError::from(problem))
}
