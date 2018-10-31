use bitcoin_support::{self, BitcoinQuantity, PubkeyHash, BTC_BLOCKS_IN_24H};
use comit_client::{self, SwapReject, SwapResponseError};
use ethereum_support::{self, EtherQuantity};
use event_store::{self, EventStore};
use futures::{sync::mpsc::UnboundedSender, Future, Stream};
use http_api_problem::HttpApiProblem;
use hyper::{header, rt::Stream as HyperStream, Body, Response, StatusCode};
use key_store::KeyStore;
use mime;
use rand::OsRng;
use serde_json;
use std::{
    net::SocketAddr,
    ops::DerefMut,
    panic::RefUnwindSafe,
    sync::{Arc, Mutex},
};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{self, bitcoin, Secret},
};
use swaps::{alice_events, common::TradeId};
use tokio;
use warp::{self, Rejection, Reply};
use warp_factory::SwapState;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ClientFactory(comit_client::ClientFactoryError),
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

impl From<comit_client::ClientFactoryError> for Error {
    fn from(e: comit_client::ClientFactoryError) -> Self {
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
    Ether { quantity: EtherQuantity },
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

pub fn post_swap<
    C: comit_client::Client + 'static,
    F: comit_client::ClientFactory<C> + 'static,
    E: event_store::EventStore<TradeId> + RefUnwindSafe,
>(
    swap_state: SwapState,
    client_factory: Arc<F>,
    event_store: Arc<E>,
    swap: Swap,
) -> Result<impl Reply, Rejection> {
    let result = {
        handle_post_swap(
            swap,
            &event_store,
            &swap_state.rng,
            &client_factory,
            swap_state.remote_comit_node_socket_addr,
            &swap_state.key_store,
            &swap_state.alice_actor_sender,
        )
    };
    match result {
        Ok(swap_created) => {
            let response = Response::builder()
                .status(StatusCode::CREATED)
                .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .header(header::LOCATION, format!("/swaps/{}", swap_created.id))
                .body(Body::from(
                    serde_json::to_string(&swap_created).expect("should always serialize"),
                )).unwrap();
            Ok(response)
        }
        Err(e) => {
            error!("Problem with sending swap request: {:?}", e);
            Err(warp::reject::server_error())
        }
    }
}

fn handle_post_swap<
    C: comit_client::Client,
    F: comit_client::ClientFactory<C> + 'static,
    E: EventStore<TradeId>,
>(
    swap: Swap,
    event_store: &Arc<E>,
    rng: &Mutex<OsRng>,
    client_factory: &Arc<F>,
    comit_node_addr: SocketAddr,
    key_store: &Arc<KeyStore>,
    alice_actor_sender: &Arc<Mutex<UnboundedSender<TradeId>>>,
) -> Result<SwapCreated, Error> {
    let id = TradeId::default();
    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };
    let client = client_factory.client_for(comit_node_addr)?;

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
                        source_ledger_lock_duration,
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
                    let alice_actor_sender = alice_actor_sender.clone();

                    tokio::spawn(response_future.then(move |response| {
                        on_swap_response::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, E>(
                            id,
                            &event_store,
                            &alice_actor_sender,
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
    SL: rfc003::Ledger,
    TL: rfc003::Ledger,
    SA: Clone + Send + Sync + 'static,
    TA: Clone + Send + Sync + 'static,
    E: EventStore<TradeId>,
>(
    id: TradeId,
    event_store: &Arc<E>,
    alice_actor_sender: &Arc<Mutex<UnboundedSender<TradeId>>>,
    result: Result<Result<rfc003::AcceptResponse<SL, TL>, SwapReject>, SwapResponseError>,
) {
    match result {
        Ok(Ok(accepted)) => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestAccepted::<SL, TL, SA, TA>::new(
                        accepted.target_ledger_refund_identity,
                        accepted.source_ledger_success_identity,
                        accepted.target_ledger_lock_duration,
                    ),
                ).expect("It should not be possible to be in the wrong state");

            let mut alice_actor_sender = alice_actor_sender
                .lock()
                .expect("Issue with unlocking alice actor sender");
            let alice_actor_sender = alice_actor_sender.deref_mut();
            alice_actor_sender
                .unbounded_send(id)
                .expect("Receiver should always be in scope");
        }
        _ => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestRejected::<SL, TL, SA, TA>::new(),
                ).expect("It should not be possible to be in the wrong state");
        }
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

pub fn get_swap<E: EventStore<TradeId> + RefUnwindSafe>(
    event_store: Arc<E>,
    id: TradeId,
) -> Result<impl Reply, Rejection> {
    let result = handle_get_swap(id, &event_store);

    match result {
        Some(swap_status) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(
                serde_json::to_string(&swap_status).expect("should always serialize"),
            )).unwrap()),
        None => Err(warp::reject::bad_request()),
    }
}

fn handle_get_swap<E: EventStore<TradeId>>(
    id: TradeId,
    event_store: &Arc<E>,
) -> Option<SwapStatus> {
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
