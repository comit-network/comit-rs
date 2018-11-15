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
        state_store::{self, StateStore},
        Ledger, Secret, SecretHash,
    },
    Assets, Ledgers, Metadata, MetadataStore, Roles,
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
pub struct SwapRequestBody<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset> {
    #[serde(with = "http_api::asset::serde")]
    source_asset: SA,
    #[serde(with = "http_api::asset::serde")]
    target_asset: TA,
    #[serde(with = "http_api::ledger::serde")]
    source_ledger: SL,
    #[serde(with = "http_api::ledger::serde")]
    target_ledger: TL,
    source_ledger_refund_identity: SL::Identity,
    target_ledger_success_identity: TL::Identity,
    source_ledger_lock_duration: SL::LockDuration,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SwapCombinations {
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
    swap_request: SwapCombinations,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequests)>,
) -> Result<impl Reply, Rejection> {
    let id = SwapId::default();

    let requests = match swap_request {
        SwapCombinations::BitcoinEthereumBitcoinQuantityEthereumQuantity(body) => {
            rfc003::alice::SwapRequests::BitcoinEthereumBitcoinQuantityEthereumQuantity(
                frunk::labelled_convert_from(body),
            )
        }
    };

    if let Err(e) = sender.unbounded_send((id, requests)) {
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
            http_api_problem: Error::NotFound.into(),
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
                    let target_funded = event_store.get_event::<alice_events::TargetFunded<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EtherQuantity,
                    >>(id);

                    match target_funded {
                        Ok(target_funded) => {
                            Some(SwapStatus::Redeemable {
                                contract_address: target_funded.address,
                                data: requested.secret,
                                // TODO: check how much gas we should tell the customer to pay
                                gas: 3500,
                            })
                        }
                        Err(_) => {
                            let htlc = bitcoin::Htlc::new(
                                accepted.source_ledger_success_identity,
                                requested.source_ledger_refund_identity,
                                requested.secret.hash(),
                                requested.source_ledger_lock_duration.into(),
                            );
                            Some(SwapStatus::Accepted {
                                funding_required: htlc
                                    .compute_address(requested.source_ledger.network),
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
            source_ledger: Ledgers::Bitcoin,
            target_ledger: Ledgers::Ethereum,
            source_asset: Assets::Bitcoin,
            target_asset: Assets::Ether,
            role,
        }) => match role {
            Roles::Alice => match state_store
                .get::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, Secret>(id)
            {
                Err(e) => error!("Could not retrieve state: {:?}", e),
                Ok(state) => info!("Here is the state we have retrieved: {:?}", state),
            },
            Roles::Bob => match state_store
                .get::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash>(id)
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
                "source_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "target_ledger": {
                    "name": "Ethereum"
                },
                "source_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "target_asset": {
                    "name": "Ether",
                    "quantity": "10000000000000000000"
                },
                "source_ledger_refund_identity": "ac2db2f2615c81b83fe9366450799b4992931575",
                "target_ledger_success_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "source_ledger_lock_duration": 144
            }"#;

        let body = serde_json::from_str(body);

        assert_that(&body).is_ok_containing(SwapRequestBody {
            source_asset: BitcoinQuantity::from_bitcoin(1.0),
            target_asset: EtherQuantity::from_eth(10.0),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
            source_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "ac2db2f2615c81b83fe9366450799b4992931575",
            )
            .unwrap(),
            target_ledger_success_identity: ethereum_support::Address::from(
                "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            ),
            source_ledger_lock_duration: bitcoin_support::Blocks::new(144),
        })
    }

}
