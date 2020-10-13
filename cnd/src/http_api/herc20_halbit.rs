mod alice;
mod bob;

use crate::{
    halbit, herc20,
    http_api::{problem, Halbit, Herc20, PostBody},
    network::{swap_digest, Identities, Swarm},
    storage::{Save, Storage},
    LocalSwapId,
};
use comit::network::swap_digest::Digestable;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn post_swap(
    body: PostBody<Herc20, Halbit>,
    storage: Storage,
    swarm: Swarm,
) -> Result<impl Reply, Rejection> {
    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<herc20::CreatedSwap, halbit::CreatedSwap>(swap_id);
    storage
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let identities = Identities {
        ethereum_identity: Some(body.alpha.identity),
        lightning_identity: Some(body.beta.identity),
        bitcoin_identity: None,
    };
    let digest = swap_digest::herc20_halbit(body.clone());
    let (peer, address_hint) = body.peer.into_peer_with_address_hint();
    let role = body.role;

    swarm
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

impl From<PostBody<Herc20, Halbit>> for swap_digest::Herc20Halbit {
    fn from(body: PostBody<Herc20, Halbit>) -> Self {
        Self {
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.token_contract,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: Digestable(body.beta.amount),
        }
    }
}
