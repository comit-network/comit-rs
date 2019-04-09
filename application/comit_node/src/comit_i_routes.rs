use comit_i::Asset;
use http::{uri::PathAndQuery, Response, StatusCode};
use mime_guess::Mime;
use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display},
    str::FromStr,
};
use warp::{filters::BoxedFilter, path::Tail, Filter, Rejection, Reply};

#[derive(Copy, Clone, Debug)]
enum Error {
    IndexHtmlMissing,
    PathConversionFail,
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<Error> for Rejection {
    fn from(err: Error) -> Self {
        warp::reject::custom(err)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::IndexHtmlMissing => "index.html file not found",
            Error::PathConversionFail => "Conversion of the path to PathAndQuery failed",
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

pub fn create() -> BoxedFilter<(impl Reply,)> {
    warp::any()
        .and(warp::path::tail())
        .and_then(serve)
        .recover(unpack_problem)
        .boxed()
}

fn serve(path: Tail) -> Result<impl Reply, Rejection> {
    let path = path.as_str();
    let path_and_query = PathAndQuery::from_str(path).map_err(|e| {
        error!("Could not convert path {} to PathAndQuery: {:?}", path, e);
        Error::PathConversionFail
    })?;
    let path = path_and_query.path();

    let mut mime =
        mime_guess::guess_mime_type_opt(path).unwrap_or("text/html".parse::<Mime>().unwrap());

    let asset: Option<Cow<'static, [u8]>> = Asset::get(path);

    let file = asset
        .or_else(|| {
            mime = "text/html".parse().unwrap();
            Asset::get("index.html")
        })
        .ok_or_else(|| Error::IndexHtmlMissing)?;

    Ok(Response::builder()
        .header("content-type", mime.to_string())
        .body(file))
}

pub fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(&err) = rejection.find_cause::<Error>() {
        let code = match err {
            Error::IndexHtmlMissing => StatusCode::INTERNAL_SERVER_ERROR,
            Error::PathConversionFail => StatusCode::BAD_REQUEST,
        };
        let msg = err.to_string();

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });
        Ok(warp::reply::with_status(json, code))
    } else {
        Err(rejection)
    }
}
