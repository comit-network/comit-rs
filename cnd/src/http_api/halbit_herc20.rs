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

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(
    body: PostBody<Halbit, Herc20>,
    storage: Storage,
    swarm: Swarm,
) -> Result<impl Reply, Rejection> {
    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<halbit::CreatedSwap, herc20::CreatedSwap>(swap_id);
    storage
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let identities = Identities {
        ethereum_identity: Some(body.beta.identity),
        lightning_identity: Some(body.alpha.identity),
        bitcoin_identity: None,
    };
    let digest = swap_digest::halbit_herc20(body.clone());
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

impl From<PostBody<Halbit, Herc20>> for swap_digest::HalbitHerc20 {
    fn from(body: PostBody<Halbit, Herc20>) -> Self {
        Self {
            lightning_cltv_expiry: body.alpha.cltv_expiry.into(),
            lightning_amount: Digestable(body.alpha.amount),
            ethereum_absolute_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.token_contract,
        }
    }
}
