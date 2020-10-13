mod alice;
mod bob;

use crate::{
    hbit, herc20,
    http_api::{problem, Hbit, Herc20, PostBody},
    network::{Identities, Swarm},
    storage::{Save, Storage},
    LocalSwapId, Side,
};
use comit::network::{swap_digest, swap_digest::Digestable};
use warp::{http::StatusCode, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(
    body: PostBody<Herc20, Hbit>,
    storage: Storage,
    swarm: Swarm,
) -> Result<impl Reply, Rejection> {
    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<herc20::CreatedSwap, hbit::CreatedSwap>(swap_id);
    storage
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let role = body.role;
    let transient_key = storage.derive_transient_identity(swap_id, role, Side::Beta);

    let identities = Identities {
        ethereum_identity: Some(body.alpha.identity),
        bitcoin_identity: Some(transient_key),
        lightning_identity: None,
    };
    let digest = swap_digest::herc20_hbit(body.clone());
    let (peer, address_hint) = body.peer.into_peer_with_address_hint();

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

impl From<PostBody<Herc20, Hbit>> for swap_digest::Herc20Hbit {
    fn from(body: PostBody<Herc20, Hbit>) -> Self {
        Self {
            ethereum_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.token_contract,
            bitcoin_expiry: body.beta.absolute_expiry.into(),
            bitcoin_amount: Digestable(body.beta.amount),
        }
    }
}
