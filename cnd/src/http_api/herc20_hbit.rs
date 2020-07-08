mod alice;
mod bob;

use crate::{
    hbit, herc20,
    http_api::{problem, Hbit, Herc20, PostBody},
    network::Identities,
    storage::Save,
    Facade, LocalSwapId, Side,
};
use comit::network::swap_digest;
use serde::Deserialize;
use warp::{http::StatusCode, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(body: serde_json::Value, facade: Facade) -> Result<impl Reply, Rejection> {
    let body = PostBody::<Herc20, Hbit>::deserialize(&body)
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
    let digest = swap_digest::herc20_hbit(body.clone());
    let (peer, address_hint) = body.peer.into_peer_with_address_hint();

    facade
        .initiate_communication(swap_id, role, digest, identities, peer, address_hint)
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

impl From<PostBody<Herc20, Hbit>> for swap_digest::Herc20Hbit {
    fn from(body: PostBody<Herc20, Hbit>) -> Self {
        Self {
            ethereum_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.token_contract,
            bitcoin_expiry: body.beta.absolute_expiry.into(),
            bitcoin_amount: body.beta.amount,
        }
    }
}
