use warp::path;
// Keep `use warp::path;` separate to stop cargo fmt changing it to
// `warp::path::{self}`
use crate::settings::ComitNodeSettings;
use comit_i::Asset;
use http::Response;
use mime_guess;
use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr},
};
use warp::{filters::BoxedFilter, path::Tail, Filter, Rejection, Reply};

pub fn create(settings: ComitNodeSettings) -> BoxedFilter<(impl Reply,)> {
    let settings = warp::any().map(move || settings.clone());

    let comit_node_config = path!("config" / "comitNode.js")
        .and(warp::get2())
        .and(warp::query::<GetConfigQueryParams>())
        .and(warp::path::end())
        .and(settings)
        .and_then(serve_comit_node_config);

    let comit_i = warp::any()
        .and(warp::path::tail())
        .and_then(|path: Tail| serve_comit_i_file(path.as_str()));

    comit_node_config.or(comit_i).boxed()
}

fn serve_comit_i_file(path: &str) -> Result<impl Reply, Rejection> {
    Asset::get(path)
        .map(|asset| {
            let mime = mime_guess::guess_mime_type(path);
            Response::builder()
                .header("content-type", mime.to_string())
                .body(asset)
        })
        .or_else(index_html)
        .ok_or_else(|| warp::reject::custom("index.html not found"))
}

fn index_html() -> Option<Result<Response<Cow<'static, [u8]>>, http::Error>> {
    Asset::get("index.html").map(|index| {
        Response::builder()
            .header("content-type", "text/html")
            .body(index)
    })
}

#[derive(Clone, serde::Deserialize, Debug, PartialEq)]
pub struct GetConfigQueryParams {
    callback: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComitNodeConnectionDetails {
    pub host: IpAddr,
    pub port: u16,
}

impl ComitNodeConnectionDetails {
    fn new(settings: ComitNodeSettings) -> Self {
        ComitNodeConnectionDetails {
            host: if settings.http_api.address.is_unspecified() {
                IpAddr::V4(Ipv4Addr::LOCALHOST)
            } else {
                settings.http_api.address
            },
            port: settings.http_api.port,
        }
    }
}

fn serve_comit_node_config(
    query_params: GetConfigQueryParams,
    settings: ComitNodeSettings,
) -> Result<Response<String>, Rejection> {
    let conn_details = ComitNodeConnectionDetails::new(settings);
    let conn_details = serde_json::to_string(&conn_details).map_err(|e| {
        warp::reject::custom(format!(
            "issue deserializing comit node connection details: {:?}",
            e
        ))
    })?;

    let body = format!(
        "function {}(){{ return {}; }}",
        query_params.callback, conn_details
    );
    Response::builder()
        .header("content-type", "application/javascript")
        .body(body)
        .map_err(|e| {
            warp::reject::custom(format!(
                "issue creating comit node connection details response: {:?}",
                e
            ))
        })
}
