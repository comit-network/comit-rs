use crate::{
    asset,
    db::{CreatedSwap, Save},
    ethereum::ChainId,
    halbit, hbit, herc20,
    http_api::{problem, routes::into_rejection, DialInformation, Http},
    identity,
    network::{HalbitHerc20, HbitHerc20, Herc20Halbit, Herc20Hbit, Identities, ListenAddresses},
    swap_protocols::{ledger, Rfc003Facade},
    Facade, LocalSwapId, Role, Side,
};
use chrono::Utc;
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

pub async fn post_herc20_halbit(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = Body::<Herc20, Halbit>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<herc20::CreatedSwap, halbit::CreatedSwap>(swap_id);
    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let identities = Identities {
        ethereum_identity: Some(body.alpha.identity),
        lightning_identity: Some(body.beta.identity),
        bitcoin_identity: None,
    };
    let digest = Herc20Halbit::from(body.clone()).digest();
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

#[allow(clippy::needless_pass_by_value)]
pub async fn post_halbit_herc20(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = Body::<Halbit, Herc20>::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let swap_id = LocalSwapId::default();
    let reply = warp::reply::reply();

    let swap = body.to_created_swap::<halbit::CreatedSwap, herc20::CreatedSwap>(swap_id);
    facade
        .save(swap)
        .await
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    let identities = Identities {
        ethereum_identity: Some(body.beta.identity),
        lightning_identity: Some(body.alpha.identity),
        bitcoin_identity: None,
    };
    let digest = HalbitHerc20::from(body.clone()).digest();
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

#[allow(clippy::needless_pass_by_value)]
pub async fn post_herc20_hbit(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
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

#[allow(clippy::needless_pass_by_value)]
pub async fn post_hbit_herc20(
    body: serde_json::Value,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = Body::<Hbit, Herc20>::deserialize(&body)
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
    let digest = HbitHerc20::from(body.clone()).digest();
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

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Body<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Http<Role>,
}

impl From<Body<Herc20, Halbit>> for Herc20Halbit {
    fn from(body: Body<Herc20, Halbit>) -> Self {
        Self {
            ethereum_absolute_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.contract_address,
            lightning_cltv_expiry: body.beta.cltv_expiry.into(),
            lightning_amount: body.beta.amount.0,
        }
    }
}

impl From<Body<Halbit, Herc20>> for HalbitHerc20 {
    fn from(body: Body<Halbit, Herc20>) -> Self {
        Self {
            lightning_cltv_expiry: body.alpha.cltv_expiry.into(),
            lightning_amount: body.alpha.amount.0,
            ethereum_absolute_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.contract_address,
        }
    }
}

impl From<Body<Herc20, Hbit>> for Herc20Hbit {
    fn from(body: Body<Herc20, Hbit>) -> Self {
        Self {
            ethereum_expiry: body.alpha.absolute_expiry.into(),
            erc20_amount: body.alpha.amount,
            token_contract: body.alpha.contract_address,
            bitcoin_expiry: body.beta.absolute_expiry.into(),
            bitcoin_amount: *body.beta.amount,
        }
    }
}

impl From<Body<Hbit, Herc20>> for HbitHerc20 {
    fn from(body: Body<Hbit, Herc20>) -> Self {
        Self {
            bitcoin_expiry: body.alpha.absolute_expiry.into(),
            bitcoin_amount: *body.alpha.amount,
            ethereum_expiry: body.beta.absolute_expiry.into(),
            erc20_amount: body.beta.amount,
            token_contract: body.beta.contract_address,
        }
    }
}

impl<A, B> Body<A, B> {
    fn to_created_swap<CA, CB>(&self, swap_id: LocalSwapId) -> CreatedSwap<CA, CB>
    where
        CA: From<A>,
        CB: From<B>,
        Self: Clone,
    {
        let body = self.clone();

        let alpha = CA::from(body.alpha);
        let beta = CB::from(body.beta);

        let start_of_swap = Utc::now().naive_local();

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role.0,
            start_of_swap,
        }
    }
}

/// Data for the halbit protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Halbit {
    pub amount: Http<asset::Bitcoin>,
    pub identity: identity::Lightning,
    pub network: Http<ledger::Bitcoin>,
    pub cltv_expiry: u32,
}

/// Data for the herc20 protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Herc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: ChainId,
    // TODO: This should re-named to token_contract but doing so is a breaking API change.
    pub contract_address: identity::Ethereum,
    pub absolute_expiry: u32,
}

/// Data for the hbit protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
struct Hbit {
    pub amount: Http<asset::Bitcoin>,
    // TODO: This should re-named to final_identity but doing so is a breaking API change.
    pub identity: Http<bitcoin::Address>,
    pub network: Http<bitcoin::Network>,
    pub absolute_expiry: u32,
}

impl From<Halbit> for halbit::CreatedSwap {
    fn from(p: Halbit) -> Self {
        halbit::CreatedSwap {
            asset: *p.amount,
            identity: p.identity,
            network: *p.network,
            cltv_expiry: p.cltv_expiry,
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

impl From<Hbit> for hbit::CreatedSwap {
    fn from(p: Hbit) -> Self {
        hbit::CreatedSwap {
            amount: *p.amount,
            final_identity: p.identity.0,
            network: p.network.0.into(),
            absolute_expiry: p.absolute_expiry,
        }
    }
}
