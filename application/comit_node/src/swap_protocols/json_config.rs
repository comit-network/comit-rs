use bitcoin_payment_future::LedgerServices;
use bitcoin_rpc_client::BitcoinRpcApi;
use bitcoin_support::{BitcoinQuantity, ToP2wpkhAddress};
use comit_wallet::KeyStore;
use common_types::seconds::Seconds;
use ethereum_support::{EthereumQuantity, ToEthereumAddress};
use event_store::{EventStore, InMemoryEventStore};
use futures::{Future, Stream};
use futures_ext::FutureFactory;
use ledger_query_service::{BitcoinQuery, LedgerQueryServiceApiClient};
use std::sync::Arc;
use swap_protocols::{
    handler::SwapRequestHandler,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::{self, ledger_htlc_service::EthereumService},
    wire_types::{Asset, Ledger, SwapProtocol, SwapRequestHeaders, SwapResponse},
};
use swaps::{bob_events::OrderTaken, common::TradeId};
use tokio;
use transport_protocol::{
    config::Config,
    json::{self, Request, Response},
    RequestError, Status,
};

pub fn json_config<
    H: SwapRequestHandler<rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>>
        + SwapRequestHandler<rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>>,
    //TODO: Remove 'static?
    C: 'static + LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>, //        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>
>(
    mut handler: H,
    key_store: Arc<KeyStore>,
    //TODO: should EventStore type parameter be passed as type parameter?
    event_store: Arc<InMemoryEventStore<TradeId>>,
    future_factory: Arc<FutureFactory<LedgerServices>>,
    ledger_query_service_api_client: Arc<C>,
    bitcoin_node: Arc<BitcoinRpcApi>,
    ethereum_service: Arc<EthereumService>,
) -> Config<Request, Response> {
    Config::new().on_request(
        "SWAP",
        &[
            "target_ledger",
            "source_ledger",
            "target_asset",
            "source_asset",
            "swap_protocol",
        ],
        move |request: Request| {
            let headers = SwapRequestHeaders {
                source_ledger: header!(request.get_header("source_ledger")),
                target_ledger: header!(request.get_header("target_ledger")),
                source_asset: header!(request.get_header("source_asset")),
                target_asset: header!(request.get_header("target_asset")),
                swap_protocol: header!(request.get_header("swap_protocol")),
            };

            match headers.swap_protocol {
                SwapProtocol::ComitRfc003 => match headers {
                    SwapRequestHeaders {
                        source_ledger: Ledger::Bitcoin,
                        source_asset:
                            Asset::Bitcoin {
                                quantity: source_quantity,
                            },
                        target_ledger: Ledger::Ethereum,
                        target_asset:
                            Asset::Ether {
                                quantity: target_quantity,
                            },
                        ..
                    } => {
                        let request = rfc003::Request::new(
                            Bitcoin {},
                            Ethereum {},
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        match handler.handle(request.clone()) {
                            SwapResponse::Decline => {
                                Response::new(RequestError::TradeDeclined {}.status())
                            }
                            SwapResponse::Accept => build_response(
                                request,
                                key_store.clone(),
                                event_store.clone(),
                                future_factory.clone(),
                                ledger_query_service_api_client.clone(),
                            ),
                        }
                    }
                    SwapRequestHeaders {
                        source_ledger: Ledger::Ethereum,
                        source_asset:
                            Asset::Ether {
                                quantity: source_quantity,
                            },
                        target_ledger: Ledger::Bitcoin,
                        target_asset:
                            Asset::Bitcoin {
                                quantity: target_quantity,
                            },
                        ..
                    } => {
                        let request = rfc003::Request::new(
                            Ethereum {},
                            Bitcoin {},
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        match handler.handle(request.clone()) {
                            SwapResponse::Decline => {
                                Response::new(RequestError::TradeDeclined {}.status())
                            }
                            SwapResponse::Accept => {
                                Response::new(RequestError::UnsupportedLedger {}.status())
                            }
                        }
                    }
                    _ => Response::new(Status::SE(22)), // 22 = unsupported pair or source/target combination
                },
            }
        },
    )
}

const EXTRA_DATA_FOR_TRANSIENT_REDEEM: [u8; 1] = [1];

// TODO: Probably split this in 3: save event, setup future, build response
fn build_response<E: EventStore<TradeId>, C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>(
    request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    key_store: Arc<KeyStore>,
    event_store: Arc<E>,
    future_factory: Arc<FutureFactory<LedgerServices>>,
    ledger_query_service_api_client: Arc<C>,
) -> Response {
    // TODO: need to remove confusion as bob/my are interchangeable and interchanged. See #297
    // TODO: Prefer "redeem vs refund vs final" terminology than the "success" that may be misleading
    let alice_refund_address = request.source_ledger_refund_identity.clone().into();

    let uid = TradeId::default();

    let bob_success_keypair =
        key_store.get_transient_keypair(&uid.into(), &EXTRA_DATA_FOR_TRANSIENT_REDEEM);
    let bob_success_address = bob_success_keypair
        .public_key()
        .clone()
        .to_p2wpkh_address(request.source_ledger.network())
        .into();
    debug!(
        "Generated transient success address for Bob is {}",
        bob_success_address
    );

    let bob_refund_keypair = key_store.get_new_internal_keypair();

    let bob_refund_address = bob_refund_keypair.public_key().to_ethereum_address();
    debug!(
        "Generated address for Bob's refund is {}",
        bob_refund_address
    );

    let twelve_hours = Seconds::new(60 * 60 * 12);

    let order_taken = OrderTaken::<Ethereum, Bitcoin> {
        uid,
        contract_secret_lock: request.secret_hash,
        alice_contract_time_lock: request.source_ledger_lock_duration,
        bob_contract_time_lock: twelve_hours,
        alice_refund_address,
        alice_success_address: request.target_ledger_success_identity.into(),
        bob_refund_address: bob_refund_address.clone(),
        bob_success_address,
        bob_success_keypair: bob_success_keypair.clone(),
        buy_amount: request.target_asset,
        sell_amount: request.source_asset,
    };

    if let Err(e) = event_store.add_event(order_taken.uid, order_taken.clone()) {
        error!(
            "Declining trade because of problem with event store {:?}",
            e
        );
        return json::Response::new(Status::SE(99));
    };

    //TODO: needs to be Some()
    let query = BitcoinQuery { to_address: None };

    //TODO: remove unwrap
    let query_id = ledger_query_service_api_client.create(query).unwrap();

    let stream = future_factory.create_stream_from_template(query_id);

    tokio::run(
        stream
            .take(1)
            .for_each(|tx| {
                // TODO: actually do something with the tx
                println!("Tx: {}", tx);
                Ok(())
            }).map_err(|e| {
                error!("Ledger Query Service Failure: {:#?}", e);
            }),
    );

    // TODO: probably need to put 20 in an enum?
    let response = json::Response::new(Status::OK(20));
    response.with_body(rfc003::AcceptResponse::<Bitcoin, Ethereum> {
        target_ledger_refund_identity: bob_refund_address.into(),
        source_ledger_success_identity: bob_success_keypair.public_key().clone().into(),
        target_ledger_lock_duration: twelve_hours,
    })
}
