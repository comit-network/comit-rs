use bitcoin_htlc;
use bitcoin_support::{self, BitcoinQuantity, PubkeyHash, BTC_BLOCKS_IN_24H};
use comit_client::{self, SwapReject, SwapResponseError};
use comit_wallet::KeyStore;
use common_types::secret::Secret;
use ethereum_support::{self, EthereumQuantity};
use event_store::{self, EventStore, InMemoryEventStore};
use futures::{future, Future, Stream};
use gotham::{
    handler::{HandlerFuture, IntoHandlerError},
    state::{FromState, State},
};
use gotham_factory::{ClientFactory, SwapId, SwapState};
use http_api_problem::HttpApiProblem;
use hyper::{header, Body, Response, StatusCode};
use mime;
use rand::OsRng;
use serde_json;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use swap_protocols::{
    ledger::{self, bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
};
use swaps::{alice_events, common::TradeId};
use tokio;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ClientFactory(comit_client::FactoryError),
    Unsupported,
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            ClientFactory(e) => {
                error!("Connection error: {:?}", e);
                HttpApiProblem::new("couterparty-connection-error")
                    .set_status(500)
                    .set_detail("There was a problem connecting to the counterparty")
            }
            EventStore(_e) => HttpApiProblem::with_title_and_type_from_status(500),
            Unsupported => HttpApiProblem::new("swap-not-supported").set_status(400),
        }
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        Error::EventStore(e)
    }
}

impl From<comit_client::FactoryError> for Error {
    fn from(e: comit_client::FactoryError) -> Self {
        Error::ClientFactory(e)
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "value")]
pub enum Ledger {
    Bitcoin {
        identity: bitcoin_support::PubkeyHash,
    },
    Ethereum {
        identity: ethereum_support::Address,
    },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "value")]
pub enum Asset {
    Bitcoin { quantity: BitcoinQuantity },
    Ether { quantity: EthereumQuantity },
}

#[derive(Deserialize, Debug)]
pub struct Swap {
    source_ledger: Ledger,
    target_ledger: Ledger,
    source_asset: Asset,
    target_asset: Asset,
}

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: TradeId,
}

pub fn post_swap<C: comit_client::Client + 'static>(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                //TODO: Handle json ser/de generically for all routes
                match serde_json::from_slice(valid_body.as_ref()) {
                    Ok(swap) => {
                        let result = {
                            let swap_state = SwapState::borrow_from(&state);
                            let client_factory = ClientFactory::<C>::borrow_from(&state);
                            handle_post_swap::<C>(
                                swap,
                                &swap_state.event_store,
                                &swap_state.rng,
                                &client_factory.0,
                                swap_state.remote_comit_node_socket_addr,
                                &swap_state.key_store,
                            )
                        };
                        match result {
                            Ok(swap_created) => {
                                let response = Response::builder()
                                    .status(StatusCode::CREATED)
                                    .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                                    .header(header::LOCATION, format!("/swaps/{}", swap_created.id))
                                    .body(Body::from(
                                        serde_json::to_string(&swap_created)
                                            .expect("should always serialize"),
                                    )).unwrap();
                                Ok((state, response))
                            }
                            Err(e) => {
                                error!("Problem with sending swap request: {:?}", e);
                                let problem: HttpApiProblem = e.into();
                                Ok((state, problem.into()))
                            }
                        }
                    }
                    Err(err) => {
                        let response = Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::empty())
                            .unwrap();
                        error!(
                            "POST /swap received invalid JSON {:?} {}",
                            err,
                            String::from_utf8_lossy(valid_body.as_ref())
                        );
                        Ok((state, response))
                    }
                }
            }
            Err(e) => Err((state, e.into_handler_error())),
        });
    Box::new(f)
}

pub fn handle_post_swap<C: comit_client::Client>(
    swap: Swap,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    rng: &Mutex<OsRng>,
    client_factory: &Arc<comit_client::Factory<C>>,
    comit_node_addr: SocketAddr,
    key_store: &Arc<KeyStore>,
) -> Result<SwapCreated, Error> {
    let id = TradeId::default();
    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };
    let client = client_factory.client_for(comit_node_addr)?;
    debug!("Retrieved client for {}", comit_node_addr);

    match (swap.source_ledger, swap.target_ledger) {
        (
            Ledger::Bitcoin {
                //XXX: Not used for now
                identity: _source_ledger_final_identity,
            },
            Ledger::Ethereum {
                identity: target_ledger_final_identity,
            },
        ) => {
            let source_ledger = Bitcoin::regtest();
            let target_ledger = Ethereum::default();
            match (swap.source_asset, swap.target_asset) {
                (
                    Asset::Bitcoin {
                        quantity: source_asset,
                    },
                    Asset::Ether {
                        quantity: target_asset,
                    },
                ) => {
                    let source_ledger_refund_identity: PubkeyHash = key_store
                        .get_transient_keypair(&id.into(), b"REFUND")
                        .public_key()
                        .into();
                    let target_ledger_success_identity = target_ledger_final_identity;

                    let source_ledger_lock_duration = BTC_BLOCKS_IN_24H;
                    let secret_hash = secret.hash();
                    let sent_event = alice_events::SentSwapRequest {
                        source_ledger: source_ledger.clone(),
                        target_ledger: target_ledger.clone(),
                        source_asset,
                        target_asset,
                        secret,
                        target_ledger_success_identity,
                        source_ledger_refund_identity,
                        source_ledger_lock_duration: source_ledger_lock_duration.clone(),
                    };

                    event_store.add_event(id, sent_event)?;
                    let response_future = client.send_swap_request(rfc003::Request {
                        secret_hash,
                        source_ledger_refund_identity,
                        target_ledger_success_identity,
                        source_ledger_lock_duration,
                        target_asset,
                        source_asset,
                        source_ledger,
                        target_ledger,
                    });

                    let event_store = event_store.clone();

                    tokio::spawn(response_future.then(move |response| {
                        on_swap_response::<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>(
                            id,
                            &event_store,
                            response,
                        );
                        Ok(())
                    }));
                    Ok(SwapCreated { id })
                }
                _ => Err(Error::Unsupported),
            }
        }
        _ => Err(Error::Unsupported),
    }
}

fn on_swap_response<
    SL: ledger::Ledger,
    TL: ledger::Ledger,
    SA: Clone + Send + Sync + 'static,
    TA: Clone + Send + Sync + 'static,
>(
    id: TradeId,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    result: Result<Result<rfc003::AcceptResponse<SL, TL>, SwapReject>, SwapResponseError>,
) {
    match result {
        Ok(response) => match response {
            Ok(accepted) => event_store
                .add_event(
                    id,
                    alice_events::SwapRequestAccepted::<SL, TL, SA, TA>::new(
                        accepted.target_ledger_refund_identity,
                        accepted.source_ledger_success_identity,
                        accepted.target_ledger_lock_duration,
                    ),
                ).unwrap(),
            Err(_rejected) => event_store
                .add_event(
                    id,
                    alice_events::SwapRequestRejected::<SL, TL, SA, TA>::new(),
                ).unwrap(),
        },
        // TODO: Decide what to do with transport level errors
        Err(_frame_error) => event_store
            .add_event(
                id,
                alice_events::SwapRequestRejected::<SL, TL, SA, TA>::new(),
            ).unwrap(),
    }
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

pub fn get_swap(state: State) -> Box<HandlerFuture> {
    let result = {
        let id = SwapId::borrow_from(&state).id;
        let swap_state = SwapState::borrow_from(&state);
        handle_get_swap(id, &swap_state.event_store)
    };

    let response = match result {
        Some(swap_status) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(
                serde_json::to_string(&swap_status).expect("should always serialize"),
            )).unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    };
    Box::new(future::ok((state, response)))
}

fn handle_get_swap(
    id: TradeId,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
) -> Option<SwapStatus> {
    let requested = event_store.get_event::<alice_events::SentSwapRequest<
        Bitcoin,
        Ethereum,
        BitcoinQuantity,
        EthereumQuantity,
    >>(id);

    match requested {
        Ok(requested) => {
            let accepted = event_store.get_event::<alice_events::SwapRequestAccepted<
                Bitcoin,
                Ethereum,
                BitcoinQuantity,
                EthereumQuantity,
            >>(id);
            match accepted {
                Ok(accepted) => {
                    let contract_deployed = event_store.get_event::<alice_events::ContractDeployed<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EthereumQuantity,
                    >>(id);

                    match contract_deployed {
                        Ok(contract_deployed) => {
                            Some(SwapStatus::Redeemable {
                                contract_address: contract_deployed.address,
                                data: requested.secret,
                                // TODO: check how much gas we should tell the customer to pay
                                gas: 3500,
                            })
                        }
                        Err(_) => {
                            let htlc = bitcoin_htlc::Htlc::new(
                                accepted.source_ledger_success_identity,
                                requested.source_ledger_refund_identity,
                                requested.secret.hash(),
                                requested.source_ledger_lock_duration.clone().into(),
                            );
                            Some(SwapStatus::Accepted {
                                funding_required: htlc
                                    .compute_address(requested.source_ledger.network()),
                            })
                        }
                    }
                }
                Err(_) => {
                    let rejected = event_store.get_event::<alice_events::SwapRequestRejected<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EthereumQuantity,
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
