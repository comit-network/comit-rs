use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use frunk;
use futures::sync::mpsc::UnboundedSender;
use http_api;
use http_api_problem::{HttpApiProblem, HttpStatusCode};
use hyper::{header, StatusCode};
use rustic_hal::HalResource;
use std::{error::Error as StdError, fmt, sync::Arc};
use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self,
        actions::StateActions,
        roles::{Alice, Bob},
        state_store::{self, StateStore},
        Ledger,
    },
    AssetKind, LedgerKind, Metadata, MetadataStore, RoleKind,
};
use swaps::common::SwapId;
use warp::{self, Rejection, Reply};

pub const PATH: &str = "rfc003";

type ActionName = String;

#[derive(Debug)]
pub enum Error {
    Unsupported,
    NotFound,
}

#[derive(Debug)]
pub struct HttpApiProblemStdError {
    pub http_api_problem: HttpApiProblem,
}

impl fmt::Display for HttpApiProblemStdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.http_api_problem.title)
    }
}

impl StdError for HttpApiProblemStdError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            Unsupported => HttpApiProblem::new("swap-not-supported").set_status(400),
            NotFound => HttpApiProblem::new("swap-not-found").set_status(404),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, LabelledGeneric)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

pub fn customize_error(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(ref err) = rejection.find_cause::<HttpApiProblemStdError>() {
        let code = err
            .http_api_problem
            .status
            .unwrap_or(HttpStatusCode::InternalServerError);
        let json = warp::reply::json(&err.http_api_problem);
        return Ok(warp::reply::with_status(
            json,
            StatusCode::from_u16(code.to_u16()).unwrap(),
        ));
    }
    Err(rejection)
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
        return Err(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: HttpApiProblem::with_title_from_status(500),
        }));
    }

    let swap_created = SwapCreated { id };
    let body = warp::reply::json(&swap_created);
    let response = warp::reply::with_header(
        body,
        header::LOCATION,
        format!("/swaps/{}", swap_created.id),
    );
    let response = warp::reply::with_status(response, warp::http::StatusCode::CREATED);

    Ok(response)
}

#[derive(Debug, Serialize)]
pub struct State {
    name: String,
    alpha_ledger: String,
    beta_ledger: String,
    alpha_asset: String,
    beta_asset: String,
    role: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    let result = handle_get_swap(&metadata_store, &state_store, &id);

    match result {
        Some((state, actions)) => {
            let mut response = HalResource::new(state);
            for action in actions {
                let route = format!("{}/{}/{}/{}", http_api::PATH, PATH, id, action);
                response.with_link("redeem", route);
            }
            Ok(warp::reply::json(&response))
        }
        None => Err(warp::reject::custom(HttpApiProblemStdError {
            http_api_problem: Error::NotFound.into(),
        })),
    }
}

fn handle_get_swap<T: MetadataStore<SwapId>, S: state_store::StateStore<SwapId>>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: &SwapId,
) -> Option<(State, Vec<ActionName>)> {
    match metadata_store.get(&id) {
        Err(e) => {
            debug!("Could not retrieve metadata: {:?}", e);
            None
        }
        Ok(
            metadata @ Metadata {
                alpha_ledger: LedgerKind::Bitcoin,
                beta_ledger: LedgerKind::Ethereum,
                alpha_asset: AssetKind::Bitcoin,
                beta_asset: AssetKind::Ether,
                ..
            },
        ) => {
            info!("Fetched metadata of swap with id {}: {:?}", id, metadata);
            match metadata.role {
                RoleKind::Alice => {
                    match state_store
                        .get::<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
                    {
                        Err(e) => {
                            error!("Could not retrieve state: {:?}", e);
                            None
                        }
                        Ok(state) => {
                            info!("Here is the state we have retrieved: {:?}", state);

                            let actions: Vec<ActionName> =
                                state.actions().iter().map(|action| action.name()).collect();
                            Some((
                                State {
                                    name: state.name(),
                                    alpha_ledger: format!("{}", metadata.alpha_ledger),
                                    beta_ledger: format!("{}", metadata.beta_ledger),
                                    alpha_asset: format!("{}", metadata.alpha_asset),
                                    beta_asset: format!("{}", metadata.beta_asset),
                                    role: format!("{}", metadata.role),
                                },
                                actions,
                            ))
                        }
                    }
                }
                RoleKind::Bob => {
                    match state_store
                        .get::<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
                    {
                        Err(e) => {
                            error!("Could not retrieve state: {:?}", e);
                            None
                        }
                        Ok(state) => {
                            info!("Here is the state we have retrieved: {:?}", state);

                            let actions: Vec<ActionName> =
                                state.actions().iter().map(|action| action.name()).collect();
                            Some((
                                State {
                                    name: state.name(),
                                    alpha_ledger: format!("{}", metadata.alpha_ledger),
                                    beta_ledger: format!("{}", metadata.beta_ledger),
                                    alpha_asset: format!("{}", metadata.alpha_asset),
                                    beta_asset: format!("{}", metadata.beta_asset),
                                    role: format!("{}", metadata.role),
                                },
                                actions,
                            ))
                        }
                    }
                }
            }
        }
        _ => unreachable!("No other type is expected to be found in the store"),
    }
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
