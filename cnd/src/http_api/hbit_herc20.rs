pub mod alice;
pub mod bob;

use crate::{
    hbit, herc20,
    http_api::{problem, Hbit, Herc20, PostBody},
    network::{swap_digest, Identities},
    storage::Save,
    Facade, LocalSwapId, Side,
};
use serde::Deserialize;
use warp::{http::StatusCode, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(body: serde_json::Value, facade: Facade) -> Result<impl Reply, Rejection> {
    let body = PostBody::<Hbit, Herc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<hbit::CreatedSwap, herc20::CreatedSwap>(swap_id);
    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let role = body.role.0;
    let transient_identity = facade
        .storage
        .derive_transient_identity(swap_id, role, Side::Alpha);

    let identities = Identities {
        bitcoin_identity: Some(transient_identity),
        ethereum_identity: Some(body.beta.identity),
        lightning_identity: None,
    };
    let digest = swap_digest::hbit_herc20(body.clone());
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

impl From<PostBody<Hbit, Herc20>> for swap_digest::HbitHerc20 {
    fn from(body: PostBody<Hbit, Herc20>) -> Self {
        Self {
            bitcoin_expiry: body.alpha.absolute_expiry.into(),
            bitcoin_amount: body.alpha.amount,
            ethereum_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.token_contract,
        }
    }
}
