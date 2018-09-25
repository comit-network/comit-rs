use bitcoin_support::{self, BTC_BLOCKS_IN_24H, BitcoinQuantity, Blocks};
use comit_client::{self, FakeClient};
use common_types::secret::Secret;
use ethereum_support::{self, EthereumQuantity};
use event_store::{self, EventStore, InMemoryEventStore};
use futures::{future, Future, Stream};
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
};
use gotham::{
    handler::{HandlerFuture, IntoHandlerError},
    state::{FromState, State},
};
use gotham_factory::{ClientFactory, SwapId, SwapState};
use http_api_problem::HttpApiProblem;
use hyper::{header::ContentType, Body, Response, StatusCode};
use rand::OsRng;
use rocket_contrib::Json;
use serde_json;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use swaps::{alice_events, common::TradeId};
use tokio;

#[derive(Debug)]
pub enum SwapError {
    EventStore(event_store::Error),
    ClientFactory(comit_client::FactoryError),
    Unsupported,
}

impl From<SwapError> for HttpApiProblem {
    fn from(e: SwapError) -> Self {
        use self::SwapError::*;
        match e {
            ClientFactory(e) => {
                error!("Connection error: {:?}", e);
                HttpApiProblem::new("couterparty-connection-error")
                    .set_status(500)
                    .set_detail("There was a problem with the connection to the counterparty")
            }
            EventStore(e) => HttpApiProblem::with_title_and_type_from_status(500),
            Unsupported => HttpApiProblem::new("swap-not-supported").set_status(400),
        }
    }
}

impl From<event_store::Error> for SwapError {
    fn from(e: event_store::Error) -> Self {
        SwapError::EventStore(e)
    }
}

impl From<comit_client::FactoryError> for SwapError {
    fn from(e: comit_client::FactoryError) -> Self {
        SwapError::ClientFactory(e)
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
                            )
                        };
                        match result {
                            Ok(swap_created) => {
                                let response = Response::new()
                                    .with_status(StatusCode::Created)
                                    .with_header(ContentType::json())
                                    .with_body(serde_json::to_string(&swap_created).unwrap());
                                Ok((state, response))
                            }
                            Err(e) => {
                                let http_problem: HttpApiProblem = e.into();
                                // (state, http_problem.to_hyper_response())
                                unimplemented!()
                            }
                        }
                    }
                    Err(err) => unimplemented!(),
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
) -> Result<SwapCreated, SwapError> {
    let id = TradeId::default();
    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };
    let client = client_factory.client_for(comit_node_addr)?;

    match (swap.source_ledger, swap.target_ledger) {
        (
            Ledger::Bitcoin {
                identity: source_ledger_refund_identity,
            },
            Ledger::Ethereum {
                identity: target_ledger_success_identity,
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
                    let source_ledger_lock_duration = BTC_BLOCKS_IN_24H;
                    let secret_hash = secret.hash();
                    let sent_event = alice_events::SentSwapRequest {
                        source_ledger: source_ledger.clone(),
                        target_ledger: target_ledger.clone(),
                        source_asset,
                        target_asset,
                        secret,
                        target_ledger_success_identity: target_ledger_success_identity.clone(),
                        source_ledger_refund_identity: source_ledger_refund_identity.clone(),
                        source_ledger_lock_duration: source_ledger_lock_duration.clone(),
                    };

                    event_store.add_event(id, sent_event)?;
                    let future = client.send_swap_request(rfc003::Request {
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

                    tokio::spawn(future.then(move |result| {
                        match result {
                            Ok(response) => match response {
                                Ok(accepted) => event_store
                                    .add_event(
                                        id,
                                        alice_events::SwapRequestAccepted::<
                                            Bitcoin,
                                            Ethereum,
                                            BitcoinQuantity,
                                            EthereumQuantity,
                                        >::new(
                                            accepted.target_ledger_refund_identity,
                                            accepted.source_ledger_success_identity,
                                            accepted.target_ledger_lock_duration,
                                        ),
                                    )
                                    .unwrap(),
                                Err(rejected) => event_store
                                    .add_event(
                                        id,
                                        alice_events::SwapRequestRejected::<
                                            Bitcoin,
                                            Ethereum,
                                            BitcoinQuantity,
                                            EthereumQuantity,
                                        >::new(),
                                    )
                                    .unwrap(),
                            },
                            Err(frame_error) => event_store
                                .add_event(
                                    id,
                                    alice_events::SwapRequestRejected::<
                                        Bitcoin,
                                        Ethereum,
                                        BitcoinQuantity,
                                        EthereumQuantity,
                                    >::new(),
                                )
                                .unwrap(),
                        }
                        Ok(())
                    }));
                    Ok(SwapCreated { id })
                }
                _ => return Err(SwapError::Unsupported),
            }
        }
        _ => return Err(SwapError::Unsupported),
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "status")]
enum SwapStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "redeemable")]
    Redeemable,
}

pub fn get_swap(state: State) -> Box<HandlerFuture> {
    let result = {
        let id = SwapId::borrow_from(&state).id;
        let swap_state = SwapState::borrow_from(&state);
        handle_get_swap(id, &swap_state.event_store)
    };

    let response = match result {
        Some(swap_status) => Response::new()
            .with_status(StatusCode::Ok)
            .with_header(ContentType::json())
            .with_body(serde_json::to_string(&swap_status).unwrap()),
        None => Response::new().with_status(StatusCode::NotFound),
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
        Ok(_requested) => {
            let accepted = event_store.get_event::<alice_events::SwapRequestAccepted<
                Bitcoin,
                Ethereum,
                BitcoinQuantity,
                EthereumQuantity,
            >>(id);

            match accepted {
                Ok(_accepted) => {
                    let contract_deployed = event_store.get_event::<alice_events::ContractDeployed<
                        Bitcoin,
                        Ethereum,
                        BitcoinQuantity,
                        EthereumQuantity,
                    >>(id);

                    match contract_deployed {
                        Ok(_deployed) => Some(SwapStatus::Redeemable),
                        Err(_) => Some(SwapStatus::Accepted),
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
        _ => unreachable!(),
    }
}

// pub fn post_swap(
//     _swap: Json<Swap>,
//     rng: State<Mutex<OsRng>>,
//     event_store: State<Arc<InMemoryEventStore<TradeId>>>,
//     client_factory: State<Arc<comit_client::Factory<FakeClient>>>,
//     remote_comit_node_socket_addr: State<SocketAddr>,
// ) -> Result<status::Created<Json<SwapCreated>>, HttpApiProblem> {
//     let swap_created = handle_swap(
//         _swap.into_inner(),
//         rng.inner(),
//         event_store.inner(),
//         *remote_comit_node_socket_addr,
//         client_factory.inner(),
//     )?;

//     Ok(status::Created(
//         format!("/swap/{}", swap_created.id),
//         Some(Json(swap_created)),
//     ))
// }

// fn handle_swap(
//     swap: Swap,
//     rng: &Mutex<OsRng>,
//     event_store: &Arc<InMemoryEventStore<TradeId>>,
//     comit_node_addr: SocketAddr,
//     client_factory: &Arc<comit_client::Factory<FakeClient>>,
// ) -> Result<SwapCreated, SwapError> {

// }

//fn make_swap_request<SL: Ledger, TL: Ledger, SA, TA>
