use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use frunk;
use futures::sync::mpsc::UnboundedSender;
use http_api::{
    self,
    problem::{self, HttpApiProblemStdError},
};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::sync::Arc;

use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self,
        actions::{Action, StateActions},
        roles::{Alice, Bob},
        state_store::StateStore,
        Ledger,
    },
    AssetKind, LedgerKind, Metadata, MetadataStore, RoleKind,
};
use swaps::common::SwapId;
use warp::{self, Rejection, Reply};

pub const PROTOCOL_NAME: &str = "rfc003";

#[derive(Clone, Debug, Deserialize, PartialEq, LabelledGeneric)]
pub struct SwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    #[serde(with = "http_api::asset::serde")]
    alpha_asset: AA,
    #[serde(with = "http_api::asset::serde")]
    beta_asset: BA,
    #[serde(with = "http_api::ledger::serde")]
    alpha_ledger: AL,
    #[serde(with = "http_api::ledger::serde")]
    beta_ledger: BL,
    alpha_ledger_refund_identity: AL::Identity,
    beta_ledger_success_identity: BL::Identity,
    alpha_ledger_lock_duration: AL::LockDuration,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SwapRequestBodyKind {
    BitcoinEthereumBitcoinQuantityEthereumQuantity(
        SwapRequestBody<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    ),
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, PROTOCOL_NAME, id)
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap(
    request_body_kind: SwapRequestBodyKind,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)>,
) -> Result<impl Reply, Rejection> {
    let id = SwapId::default();

    let request_kind = match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityEthereumQuantity(body) => {
            rfc003::alice::SwapRequestKind::BitcoinEthereumBitcoinQuantityEthereumQuantity(
                frunk::labelled_convert_from(body),
            )
        }
    };

    if let Err(e) = sender.unbounded_send((id, request_kind)) {
        error!(
            "Swap request {:?} for id {} could not dispatched.",
            e.into_inner(),
            id
        );
        return Err(warp::reject::custom(HttpApiProblemStdError::from(
            HttpApiProblem::with_title_from_status(500),
        )));
    }

    let swap_created = SwapCreated { id };
    let body = warp::reply::json(&swap_created);
    let response = warp::reply::with_header(body, header::LOCATION, swap_path(id));
    let response = warp::reply::with_status(response, warp::http::StatusCode::CREATED);

    Ok(response)
}

#[derive(Debug, Serialize)]
pub struct SwapDescription {
    alpha_ledger: String,
    beta_ledger: String,
    alpha_asset: String,
    beta_asset: String,
}

#[derive(Debug, Serialize)]
struct GetSwapResource {
    pub swap: SwapDescription,
    pub role: String,
    pub state: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    let result = handle_get_swap(&metadata_store, &state_store, &id);

    match result {
        Ok((swap_resource, actions)) => {
            let mut response = HalResource::new(swap_resource);
            for action in actions {
                let route = format!("{}/{}", swap_path(id), action);
                response.with_link(action, route);
            }
            Ok(warp::reply::json(&response))
        }
        Err(e) => Err(warp::reject::custom(HttpApiProblemStdError::new(e))),
    }
}

type ActionName = String;

fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: &SwapId,
) -> Result<(GetSwapResource, Vec<ActionName>), HttpApiProblem> {
    let metadata = metadata_store.get(id)?.ok_or_else(problem::not_found)?;
    get_swap!(
        &metadata,
        state_store,
        id,
        state,
        (|| {
            let state = state.ok_or_else(problem::not_found)?;
            trace!("Retrieved state for {}: {:?}", id, state);

            let actions: Vec<ActionName> = state.actions().iter().map(Action::name).collect();
            (Ok((
                GetSwapResource {
                    state: state.name(),
                    swap: SwapDescription {
                        alpha_ledger: format!("{}", metadata.alpha_ledger),
                        beta_ledger: format!("{}", metadata.beta_ledger),
                        alpha_asset: format!("{}", metadata.alpha_asset),
                        beta_asset: format!("{}", metadata.beta_asset),
                    },
                    role: format!("{}", metadata.role),
                },
                actions,
            )))
        })
    )
}

#[derive(Serialize, Debug)]
pub struct EmbeddedSwapResource {
    state: String,
    protocol: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    match handle_get_swaps(metadata_store, state_store) {
        Ok(swaps) => {
            let mut response = HalResource::new("");
            response.with_resources("swaps", swaps);
            Ok(warp::reply::json(&response))
        }
        Err(e) => Err(warp::reject::custom(HttpApiProblemStdError::new(e))),
    }
}

fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<Vec<HalResource>, HttpApiProblem> {
    let mut resources = vec![];
    for (id, metadata) in metadata_store.all()?.into_iter() {
        get_swap!(
            &metadata,
            &state_store,
            &id,
            state,
            (|| -> Result<(), HttpApiProblem> {
                match state {
                    Some(state) => {
                        let swap = EmbeddedSwapResource {
                            state: state.name(),
                            protocol: PROTOCOL_NAME.into(),
                        };

                        let mut hal_resource = HalResource::new(swap);
                        hal_resource.with_link("self", swap_path(id));
                        resources.push(hal_resource);
                    }
                    None => error!("Couldn't find state for {} despite having the metadata", id),
                };
                Ok(())
            })
        )?;
    }

    Ok(resources)
}

#[cfg(test)]
mod tests {

    use super::*;
    use hex::FromHex;
    use serde_json;
    use spectral::prelude::*;

    #[test]
    fn can_deserialize_swap_request_body() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "Ethereum"
                },
                "alpha_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "Ether",
                    "quantity": "10000000000000000000"
                },
                "alpha_ledger_refund_identity": "ac2db2f2615c81b83fe9366450799b4992931575",
                "beta_ledger_success_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_ledger_lock_duration": 144
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger: Bitcoin::regtest(),
            beta_ledger: Ethereum::default(),
            alpha_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "ac2db2f2615c81b83fe9366450799b4992931575",
            )
            .unwrap(),
            beta_ledger_success_identity: ethereum_support::Address::from(
                "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            ),
            alpha_ledger_lock_duration: bitcoin_support::Blocks::new(144),
        })
    }

}
