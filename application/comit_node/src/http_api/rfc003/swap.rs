use bitcoin_support::{self, BitcoinQuantity};
use comit_client;
use ethereum_support::{self, EtherQuantity};
use event_store::{self, EventStore};
use frunk;
use futures::sync::mpsc::UnboundedSender;
use http_api;
use http_api_problem::{HttpApiProblem, HttpStatusCode};
use hyper::{header, StatusCode};
use std::{error::Error as StdError, fmt, sync::Arc};
use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, bitcoin,
        roles::{Alice, Bob},
        state_store::{self, StateStore},
        Ledger, Secret,
    },
    AssetKind, LedgerKind, Metadata, MetadataStore, RoleKind,
};
use swaps::{alice_events, common::SwapId};
use warp::{self, Rejection, Reply};

pub const PATH: &str = "rfc003";

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ClientFactory(comit_client::ClientFactoryError),
    Unsupported,
    NotFound,
}

#[derive(Debug)]
pub struct HttpApiProblemStdError {
    pub inner: HttpApiProblem,
}

impl From<HttpApiProblem> for HttpApiProblemStdError {
    fn from(problem: HttpApiProblem) -> Self {
        Self { inner: problem }
    }
}

impl fmt::Display for HttpApiProblemStdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.title)
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
            ClientFactory(e) => {
                error!("Connection error: {:?}", e);
                HttpApiProblem::new("counterparty-connection-error")
                    .set_status(500)
                    .set_detail("There was a problem connecting to the counterparty")
            }
            EventStore(_e) => HttpApiProblem::with_title_and_type_from_status(500),
            Unsupported => HttpApiProblem::new("swap-not-supported").set_status(400),
            NotFound => HttpApiProblem::new("swap-not-found").set_status(404),
        }
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        Error::EventStore(e)
    }
}

impl From<comit_client::ClientFactoryError> for Error {
    fn from(e: comit_client::ClientFactoryError) -> Self {
        Error::ClientFactory(e)
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
            .inner
            .status
            .unwrap_or(HttpStatusCode::InternalServerError);
        let json = warp::reply::json(&err.inner);
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
            inner: HttpApiProblem::with_title_from_status(500),
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

#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "status")]
enum SwapStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted {
        funding_required: bitcoin_support::Address,
    },
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "redeemable")]
    Redeemable {
        contract_address: ethereum_support::Address,
        data: Secret,
        gas: u64,
    },
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<E: EventStore<SwapId>, T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    event_store: Arc<E>,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    let metadata = metadata_store.get(&id);
    info!("Fetched metadata of swap with id {}: {:?}", id, metadata);

    let result = handle_get_swap(id, &event_store, &metadata_store, &state_store);

    match result {
        Some(swap_status) => Ok(warp::reply::json(&swap_status)),
        None => Err(warp::reject::custom(HttpApiProblemStdError {
            inner: Error::NotFound.into(),
        })),
    }
}

fn handle_get_swap<E: EventStore<SwapId>, T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    id: SwapId,
    event_store: &Arc<E>,
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
) -> Option<SwapStatus> {
    {
        handle_state_for_get_swap(metadata_store, state_store, &id);
    }

    let requested = event_store.get_event::<alice_events::SentSwapRequest<
        Bitcoin,
        Ethereum,
        BitcoinQuantity,
        EtherQuantity,
    >>(id);

    match requested {
        Ok(requested) => {
            let accepted = event_store.get_event::<alice_events::SwapRequestAccepted<
                Bitcoin,
                Ethereum,
                BitcoinQuantity,
                EtherQuantity,
            >>(id);
            match accepted {
                Ok(accepted) => {
                    let beta_funded = event_store.get_event::<alice_events::BetaFunded<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EtherQuantity,
                    >>(id);

                    match beta_funded {
                        Ok(beta_funded) => {
                            Some(SwapStatus::Redeemable {
                                contract_address: beta_funded.address,
                                data: requested.secret,
                                // TODO: check how much gas we should tell the customer to pay
                                gas: 3500,
                            })
                        }
                        Err(_) => {
                            let htlc = bitcoin::Htlc::new(
                                accepted.alpha_ledger_success_identity,
                                requested.alpha_ledger_refund_identity,
                                requested.secret.hash(),
                                requested.alpha_ledger_lock_duration.into(),
                            );
                            Some(SwapStatus::Accepted {
                                funding_required: htlc
                                    .compute_address(requested.alpha_ledger.network),
                            })
                        }
                    }
                }
                Err(_) => {
                    let rejected = event_store.get_event::<alice_events::SwapRequestRejected<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EtherQuantity,
                    >>(id);

                    match rejected {
                        Ok(_rejected) => Some(SwapStatus::Rejected),
                        Err(_) => Some(SwapStatus::Pending),
                    }
                }
            }
        }
        Err(event_store::Error::NotFound) => None,
        _ => unreachable!(
            "The only type of error you can get from event store at this point is NotFound"
        ),
    }
}

fn handle_state_for_get_swap<T: MetadataStore<SwapId>, S: state_store::StateStore<SwapId>>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: &SwapId,
) {
    match metadata_store.get(&id) {
        Err(e) => error!("Could not retrieve metadata: {:?}", e),
        Ok(Metadata {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Ether,
            role,
        }) => match role {
            RoleKind::Alice => {
                match state_store
                    .get::<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
                {
                    Err(e) => error!("Could not retrieve state: {:?}", e),
                    Ok(state) => info!("Here is the state we have retrieved: {:?}", state),
                }
            }
            RoleKind::Bob => match state_store
                .get::<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>(id)
            {
                Err(e) => error!("Could not retrieve state: {:?}", e),
                Ok(state) => info!("Here is the state we have retrieved: {:?}", state),
            },
        },
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
