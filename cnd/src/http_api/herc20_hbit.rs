pub mod alice;
pub mod bob;

use crate::{
    hbit, herc20,
    http_api::{post::*, problem},
    network::{Herc20Hbit, Identities},
    storage::Save,
    Facade, LocalSwapId, Side,
};
use digest::Digest;
use serde::Deserialize;
use warp::{http::StatusCode, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(body: serde_json::Value, facade: Facade) -> Result<impl Reply, Rejection> {
    let body = Body::<Herc20, Hbit>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<herc20::CreatedSwap, hbit::CreatedSwap>(swap_id);
    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let role = body.role.0;
    let transient_key = facade
        .storage
        .derive_transient_identity(swap_id, role, Side::Beta);

    let identities = Identities {
        ethereum_identity: Some(body.alpha.identity),
        bitcoin_identity: Some(transient_key),
        lightning_identity: None,
    };
    let digest = Herc20Hbit::from(body.clone()).digest();
    let peer = body.peer.into();

    facade
        .initiate_communication(swap_id, peer, role, digest, identities)
        .await
        .map(|_| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/swaps/{}", swap_id)),
                StatusCode::CREATED,
            )
        })
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}
