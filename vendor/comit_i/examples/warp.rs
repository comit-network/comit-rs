use comit_i::Asset;
use http::uri::PathAndQuery;
use std::{borrow::Cow, str::FromStr};
use warp::{filters::path::Tail, Filter, Reply};

fn main() {
    let routes = warp::any()
        .and(warp::path::tail())
        .map(|tail: Tail| warp_wrap(tail.as_str()));

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}

fn warp_wrap(path: &str) -> impl Reply {
    let path_and_query = PathAndQuery::from_str(path).unwrap();

    let file: Cow<'static, [u8]> = Asset::get(path_and_query.path())
        .unwrap_or_else(|| Asset::get("index.html").expect("index.html file cannot be found"));

    match file {
        Cow::Borrowed(s) => warp::reply::html(String::from(std::str::from_utf8(s).unwrap())),
        Cow::Owned(s) => warp::reply::html(String::from(std::str::from_utf8(&s).unwrap())),
    }
}
