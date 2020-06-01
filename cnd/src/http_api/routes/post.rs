use crate::{
    asset,
    db::{CreatedSwap, Save},
    http_api::{problem, DialInformation, Http},
    identity,
    swap_protocols::{hbit, herc20},
    Facade, LocalSwapId, Role,
};
use serde::Deserialize;
use warp::Rejection;

/// POST endpoints for the hbit/herc20 protocol pair.

#[allow(clippy::needless_pass_by_value)]
pub async fn post_hbit_herc20(
    body: serde_json::Value,
    facade: Facade,
) -> Result<warp::reply::Json, Rejection>
where
    Facade: Save<CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>>,
{
    let body = Body::<Hbit, Herc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::random();
    let _reply = warp::reply::reply();

    let swap = hbit_herc20_created_swap_from_body(swap_id, body.clone());

    facade
        .save(swap.clone())
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    unimplemented!()
}

#[allow(clippy::needless_pass_by_value)]
pub async fn post_herc20_hbit(
    body: serde_json::Value,
    facade: Facade,
) -> Result<warp::reply::Json, Rejection>
where
    Facade: Save<CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>>,
{
    let body = Body::<Herc20, Hbit>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::random();
    let _reply = warp::reply::reply();

    let swap = herc20_hbit_created_swap_from_body(swap_id, body.clone());

    facade
        .save(swap.clone())
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    unimplemented!()
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
            asset: asset::Erc20::new(p.token_contract, p.amount),
            identity: p.identity,
            chain_id: p.chain_id,
            absolute_expiry: p.absolute_expiry,
        }
    }
}
