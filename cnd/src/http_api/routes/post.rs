use crate::{
    asset,
    db::{CreatedSwap, Save},
    http_api::{problem, routes::into_rejection, DialInformation, Http},
    identity,
    network::InitCommunication,
    swap_protocols::{hbit, herc20, Facade, LocalSwapId, Role},
};
use serde::Deserialize;
use warp::{http::StatusCode, Rejection, Reply};

/// POST endpoints for the hbit/herc20 protocol pair.

#[allow(clippy::needless_pass_by_value)]
pub async fn post_hbit_herc20(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection>
where
    Facade: Save<CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>>
        + InitCommunication<CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>>,
{
    let body = Body::<Hbit, Herc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::random();
    let reply = warp::reply::reply();

    let swap = hbit_herc20_created_swap_from_body(swap_id, body.clone());

    facade
        .save(swap.clone())
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    facade
        .init_communication(swap_id, swap)
        .await
        .map(|_| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/swaps/{}", swap_id)),
                StatusCode::CREATED,
            )
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn post_herc20_hbit(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection>
where
    Facade: Save<CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>>
        + InitCommunication<CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>>,
{
    let body = Body::<Herc20, Hbit>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::random();
    let reply = warp::reply::reply();

    let swap = herc20_hbit_created_swap_from_body(swap_id, body.clone());

    facade
        .save(swap.clone())
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    facade
        .init_communication(swap_id, swap)
        .await
        .map(|_| {
            warp::reply::with_status(
                warp::reply::with_header(reply, "Location", format!("/swaps/{}", swap_id)),
                StatusCode::CREATED,
            )
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

fn hbit_herc20_created_swap_from_body(
    swap_id: LocalSwapId,
    body: Body<Hbit, Herc20>,
) -> CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap> {
    CreatedSwap::<hbit::CreatedSwap, herc20::CreatedSwap> {
        swap_id,
        alpha: body.alpha.into(),
        beta: body.beta.into(),
        peer: body.peer.into(),
        address_hint: None,
        role: body.role.0,
    }
}

fn herc20_hbit_created_swap_from_body(
    swap_id: LocalSwapId,
    body: Body<Herc20, Hbit>,
) -> CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap> {
    CreatedSwap::<herc20::CreatedSwap, hbit::CreatedSwap> {
        swap_id,
        alpha: body.alpha.into(),
        beta: body.beta.into(),
        peer: body.peer.into(),
        address_hint: None,
        role: body.role.0,
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct Hbit {
    pub amount: Http<asset::Bitcoin>,
    pub identity: identity::Bitcoin,
    pub network: Http<bitcoin::Network>,
    pub absolute_expiry: u32,
}

impl From<Hbit> for hbit::CreatedSwap {
    fn from(p: Hbit) -> Self {
        hbit::CreatedSwap {
            amount: *p.amount,
            identity: p.identity,
            network: p.network.0.into(),
            absolute_expiry: p.absolute_expiry,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
struct Herc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub token_contract: identity::Ethereum,
    pub absolute_expiry: u32,
}

impl From<Herc20> for herc20::CreatedSwap {
    fn from(p: Herc20) -> Self {
        herc20::CreatedSwap {
            amount: p.amount,
            identity: p.identity,
            chain_id: p.chain_id,
            token_contract: p.token_contract,
            absolute_expiry: p.absolute_expiry,
        }
    }
}
