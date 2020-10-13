use crate::{config::Settings, ethereum, http_api::problem};
use anyhow::Result;
use futures::TryFutureExt;
use serde::Serialize;
use warp::{reply, Filter, Rejection, Reply};

/// The warp filter for listing all token contract addresses as used by cnd.
pub fn route(settings: Settings) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get().and(warp::path!("tokens")).and_then(move || {
        handler(settings.clone())
            .map_err(problem::from_anyhow)
            .map_err(warp::reject::custom)
    })
}

async fn handler(settings: Settings) -> Result<impl Reply> {
    let dai = Token {
        symbol: "DAI".to_owned(),
        address: settings.ethereum.tokens.dai,
        decimals: 18,
    };

    Ok(reply::json(&vec![dai]))
}

#[derive(Clone, Debug, Serialize)]
struct Token {
    symbol: String,
    address: ethereum::Address,
    decimals: u8,
}
