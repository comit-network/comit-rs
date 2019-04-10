use comit_i::Asset;
use http::Response;
use mime_guess;
use std::borrow::Cow;
use warp::{filters::BoxedFilter, path::Tail, Filter, Rejection, Reply};

pub fn create() -> BoxedFilter<(impl Reply,)> {
    warp::any()
        .and(warp::path::tail())
        .and_then(|path: Tail| serve(path.as_str()))
        .boxed()
}

fn serve(path: &str) -> Result<impl Reply, Rejection> {
    Asset::get(path)
        .map(|asset| {
            let mime = mime_guess::guess_mime_type(path);
            Response::builder()
                .header("content-type", mime.to_string())
                .body(asset)
        })
        .or_else(serve_index_html)
        .ok_or_else(|| warp::reject::custom("index.html not found"))
}

fn serve_index_html() -> Option<Result<Response<Cow<'static, [u8]>>, http::Error>> {
    Asset::get("index.html").map(|index| {
        Response::builder()
            .header("content-type", "text/html")
            .body(index)
    })
}
