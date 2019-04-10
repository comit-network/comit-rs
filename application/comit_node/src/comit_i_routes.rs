use comit_i::Asset;
use http::{Response, StatusCode};
use mime_guess;
use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display},
};
use warp::{
    filters::{path::FullPath, BoxedFilter},
    Filter, Rejection, Reply,
};

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
        .and(warp::path::full())
        .and_then(|path: FullPath| serve(path.as_str()))
        .recover(customize_error)
        .boxed()
}

fn serve(path: &str) -> Result<impl Reply, Rejection> {
    let mut mime = mime_guess::guess_mime_type_opt(path)
        .unwrap_or_else(|| mime::TEXT_HTML_UTF_8.to_string().parse().unwrap());

    let asset: Option<Cow<'static, [u8]>> = Asset::get(path);

    let file = asset
        .or_else(|| {
            mime = mime::TEXT_HTML_UTF_8.to_string().parse().unwrap();
            Asset::get("index.html")
        })
        .ok_or_else(|| Error::IndexHtmlMissing)?;

    Ok(Response::builder()
        .header("content-type", mime.to_string())
        .body(file))
}

pub fn customize_error(rejection: Rejection) -> Result<impl Reply, Rejection> {
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
