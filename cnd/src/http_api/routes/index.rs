use crate::{
    asset,
    db::{CreatedSwap, Save},
    http_api::{problem, routes::into_rejection, DialInformation, Http},
    identity,
    network::{Identities, ListenAddresses},
    swap_protocols::{halight, hbit, herc20, ledger, Facade, LocalSwapId, Rfc003Facade, Role},
};
use comit::network::swap_digest;
use digest::Digest;
use http_api_problem::HttpApiProblem;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

pub async fn get_info(id: PeerId, dependencies: Rfc003Facade) -> Result<impl Reply, Rejection> {
    let listen_addresses = dependencies.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

pub async fn get_info_siren(
    id: PeerId,
    dependencies: Rfc003Facade,
) -> Result<impl Reply, Rejection> {
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

pub async fn post_herc20_halight_bitcoin(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = Body::<Herc20, Halight>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap(swap_id);
    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let identities = Identities {
        ethereum_identity: Some(body.alpha.identity),
        lightning_identity: Some(body.beta.identity),
    };
    let digest = swap_digest::Herc20Halight::from(body.clone()).digest();
    let peer = body.peer.into();
    let role = body.role.0;

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

// `warp::reply::Json` is used as a return type to please the compiler
// until proper logic is implemented
#[allow(clippy::needless_pass_by_value)]
pub async fn post_halight_bitcoin_herc20(
    body: serde_json::Value,
    _facade: Facade,
) -> Result<warp::reply::Json, Rejection> {
    let _body = Body::<Halight, Herc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    tracing::error!("Lightning routes are not yet supported");
    Err(warp::reject::custom(
        HttpApiProblem::new("Route not yet supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This route is not yet supported."),
    ))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn post_herc20_hbit(
    body: serde_json::Value,
    _facade: Facade,
) -> Result<warp::reply::Json, Rejection> {
    let _body = Body::<Herc20, Hbit>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    Err(warp::reject::custom(
        HttpApiProblem::new("Route not yet supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This route is not yet supported."),
    ))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn post_hbit_herc20(
    body: serde_json::Value,
    _facade: Facade,
) -> Result<warp::reply::Json, Rejection> {
    let _body = Body::<Hbit, Halight>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    Err(warp::reject::custom(
        HttpApiProblem::new("Route not yet supported.")
            .set_status(StatusCode::BAD_REQUEST)
            .set_detail("This route is not yet supported."),
    ))
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

impl From<Body<Herc20, Halight>> for swap_digest::Herc20Halight {
    fn from(body: Body<Herc20, Halight>) -> Self {
        Self {
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.contract_address,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: body.beta.amount.0,
        }
    }
}

trait ToCreatedSwap<A, B> {
    fn to_created_swap(&self, id: LocalSwapId) -> CreatedSwap<A, B>;
}

impl ToCreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> for Body<Herc20, Halight> {
    fn to_created_swap(
        &self,
        swap_id: LocalSwapId,
    ) -> CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> {
        let body = self.clone();

        let alpha = herc20::CreatedSwap::from(body.alpha);
        let beta = halight::CreatedSwap::from(body.beta);

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
        }
    }
}

impl ToCreatedSwap<halight::CreatedSwap, herc20::CreatedSwap> for Body<Halight, Herc20> {
    fn to_created_swap(
        &self,
        swap_id: LocalSwapId,
    ) -> CreatedSwap<halight::CreatedSwap, herc20::CreatedSwap> {
        let body = self.clone();

        let alpha = halight::CreatedSwap::from(body.alpha);
        let beta = herc20::CreatedSwap::from(body.beta);

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
        }
    }
}

impl ToCreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap> for Body<Herc20, Hbit> {
    fn to_created_swap(
        &self,
        swap_id: LocalSwapId,
    ) -> CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap> {
        let body = self.clone();

        let alpha = herc20::CreatedSwap::from(body.alpha);
        let beta = hbit::CreatedSwap::from(body.beta);

        CreatedSwap::<herc20::CreatedSwap, hbit::CreatedSwap> {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
        }
    }
}

impl ToCreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap> for Body<Hbit, Herc20> {
    fn to_created_swap(
        &self,
        swap_id: LocalSwapId,
    ) -> CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap> {
        let body = self.clone();

        let alpha = hbit::CreatedSwap::from(body.alpha);
        let beta = herc20::CreatedSwap::from(body.beta);

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
        }
    }
}

impl From<Herc20> for herc20::CreatedSwap {
    fn from(p: Herc20) -> Self {
        herc20::CreatedSwap {
            asset: asset::Erc20::new(p.contract_address, p.amount),
            identity: p.identity,
            chain_id: p.chain_id,
            absolute_expiry: p.absolute_expiry,
        }
    }
}

/// Data for the halight protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Halight {
    pub amount: Http<asset::Bitcoin>,
    pub identity: identity::Lightning,
    pub network: Http<ledger::Lightning>,
    pub cltv_expiry: u32,
}

impl From<Halight> for halight::CreatedSwap {
    fn from(p: Halight) -> Self {
        halight::CreatedSwap {
            asset: *p.amount,
            identity: p.identity,
            network: *p.network,
            cltv_expiry: p.cltv_expiry,
        }
    }
}

/// Data for the herc20 protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Herc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub contract_address: identity::Ethereum,
    pub absolute_expiry: u32,
}

/// Data for the hbit protocol, wrapped where needed to control
/// serialization/deserialization.
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
