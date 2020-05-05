mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    asset,
    http_api::{problem, routes::into_rejection, Http},
    identity,
    network::{DialInformation, ListenAddresses},
    swap_protocols::{CreateSwapParams, Facade, Facade2, NodeLocalSwapId, Role},
};
use http_api_problem::HttpApiProblem;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

pub async fn get_info(id: PeerId, dependencies: Facade) -> Result<impl Reply, Rejection> {
    let listen_addresses = dependencies.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

pub async fn get_info_siren(id: PeerId, dependencies: Facade) -> Result<impl Reply, Rejection> {
    let listen_addresses = dependencies.listen_addresses().await.to_vec();

    Ok(warp::reply::json(
        &siren::Entity::default()
            .with_properties(&InfoResource {
                id: Http(id),
                listen_addresses,
            })
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })
            .map_err(into_rejection)?
            .with_link(
                siren::NavigationalLink::new(&["collection"], "/swaps").with_class_member("swaps"),
            )
            .with_link(
                siren::NavigationalLink::new(&["collection", "edit"], "/swaps/rfc003")
                    .with_class_member("swaps")
                    .with_class_member("rfc003"),
            ),
    ))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swaps(dependencies: Facade) -> Result<impl Reply, Rejection> {
    handle_get_swaps(dependencies)
        .await
        .map(|swaps| {
            Ok(warp::reply::with_header(
                warp::reply::json(&swaps),
                "content-type",
                "application/vnd.siren+json",
            ))
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

// `warp::reply::Json` is used as a return type to please the compiler
// until proper logic is implemented
#[allow(clippy::needless_pass_by_value)]
pub async fn post_lightning_route() -> Result<warp::reply::Json, Rejection> {
    tracing::error!("Lightning routes are not yet supported");
    Err(warp::reject::custom(
        HttpApiProblem::new("Route not yet supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This route is not yet supported."),
    ))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn post_lightning_route_new(
    body: serde_json::Value,
    facade: Facade2,
) -> Result<impl Reply, Rejection> {
    let body = Body::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let reply = warp::reply::reply();

    let id = NodeLocalSwapId::default();

    facade.save(id, ()).await;

    facade.initiate_communication(id, body.into()).await;

    Ok(warp::reply::with_status(
        warp::reply::with_header(reply, "Location", format!("/swaps/{}", id)),
        StatusCode::CREATED,
    ))
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body {
    pub alpha: HanEthereumEther,
    pub beta: HalightLightningBitcoin,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

impl From<Body> for CreateSwapParams {
    fn from(body: Body) -> Self {
        Self {
            role: body.role.0,
            peer: body.peer,
            ethereum_identity: body.alpha.identity.into(),
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            ethereum_amount: body.alpha.amount,
            lightning_identity: body.beta.identity,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: body.beta.amount.0,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct HanEthereumEther {
    pub amount: asset::Ether,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub absolute_expiry: u32,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct HalightLightningBitcoin {
    pub amount: Http<asset::Lightning>,
    pub identity: identity::Lightning,
    pub network: String,
    pub cltv_expiry: u32,
}
