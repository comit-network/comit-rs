use bitcoin_htlc;
use bitcoin_payment_future::LedgerServices;
use bitcoin_support::{Address as BitcoinAddress, BitcoinQuantity, IntoP2wpkhAddress, Network};
use comit_wallet::KeyStore;
use common_types::seconds::Seconds;
use ethereum_support::{EthereumQuantity, ToEthereumAddress};
use event_store::EventStore;
use failure::Error;
use futures::{Future, Stream};
use futures_ext::FutureFactory;
use ledger_query_service::{BitcoinQuery, LedgerQueryServiceApiClient};
use std::{sync::Arc, time::Duration};
use swap_protocols::{
    handler::SwapRequestHandler,
    ledger::{
        bitcoin::{Bitcoin, HtlcId},
        ethereum::Ethereum,
        Ledger,
    },
    rfc003::{
        self,
        ledger_htlc_service::{EtherHtlcParams, EthereumService, LedgerHtlcService},
    },
    wire_types::{SwapProtocol, SwapRequestHeaders, SwapResponse},
};
use swaps::{
    bob_events::{ContractDeployed, OrderTaken, TradeFunded},
    common::TradeId,
};
use tokio;
use transport_protocol::{
    config::Config,
    json::{self, Request, Response},
    Status,
};

pub fn json_config<
    H: SwapRequestHandler<rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>>
        + SwapRequestHandler<rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>>,
    E: EventStore<TradeId>,
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>, //TODO: when integrating Ethereum LQS + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>
>(
    mut handler: H,
    key_store: Arc<KeyStore>,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_network: Network,
    bitcoin_poll_interval: Duration,
) -> Config<Request, Response> {
    Config::default().on_request(
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

            // Too many things called Ledger so just import this on to this local namespace
            use swap_protocols::wire_types::{Asset, Ledger};

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
                            Bitcoin::default(),
                            Ethereum::default(),
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        match handler.handle(request.clone()) {
                            SwapResponse::Decline => Response::new(Status::SE(21)),
                            SwapResponse::Accept => process(
                                request,
                                &key_store,
                                event_store.clone(),
                                ledger_query_service_api_client.clone(),
                                ethereum_service.clone(),
                                bitcoin_network,
                                bitcoin_poll_interval,
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
                            Ethereum::default(),
                            Bitcoin::default(),
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        match handler.handle(request.clone()) {
                            SwapResponse::Decline => Response::new(Status::SE(21)),
                            SwapResponse::Accept => Response::new(Status::SE(22)),
                        }
                    }
                    _ => Response::new(Status::SE(22)), // 22 = unsupported pair or source/target combination
                },
            }
        },
    )
}

const EXTRA_DATA_FOR_TRANSIENT_REDEEM: [u8; 1] = [1];

fn process<E: EventStore<TradeId>, C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>>(
    request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    key_store: &Arc<KeyStore>,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_network: Network,
    bitcoin_poll_interval: Duration,
) -> Response {
    let alice_refund_address: BitcoinAddress = request
        .source_ledger
        .address_for_identity(request.source_ledger_refund_identity);

    let uid = TradeId::default();

    let bob_success_keypair =
        key_store.get_transient_keypair(&uid.into(), &EXTRA_DATA_FOR_TRANSIENT_REDEEM);
    let bob_success_address: BitcoinAddress = bob_success_keypair
        .public_key()
        .into_p2wpkh_address(request.source_ledger.network());
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
        contract_secret_lock: request.secret_hash.clone(),
        alice_contract_time_lock: request.source_ledger_lock_duration,
        bob_contract_time_lock: twelve_hours,
        alice_refund_address: alice_refund_address.clone(),
        alice_success_address: request.target_ledger_success_identity,
        bob_refund_address,
        bob_success_address: bob_success_address.clone(),
        bob_success_keypair,
        buy_amount: request.target_asset,
        sell_amount: request.source_asset,
    };

    if let Err(e) = event_store.add_event(order_taken.uid, order_taken.clone()) {
        error!(
            "Declining trade because of problem with event store: {:?}",
            e
        );
        return json::Response::new(Status::RE(0));
    };

    let btc_lock_time = (60 * 24) / 10;

    let htlc = bitcoin_htlc::Htlc::new(
        bob_success_address,
        alice_refund_address,
        request.secret_hash,
        btc_lock_time,
    );

    let query = BitcoinQuery {
        to_address: Some(htlc.compute_address(bitcoin_network)),
    };

    let query_id = match ledger_query_service_api_client.clone().create(query) {
        Ok(query_id) => query_id,
        Err(e) => {
            error!(
                "Declining trade because of problem with Bitcoin Ledger Query Service: {:?}",
                e
            );
            return json::Response::new(Status::RE(0));
        }
    };

    let ledger_services = LedgerServices::new(
        ledger_query_service_api_client.clone(),
        bitcoin_poll_interval,
    );

    let future_factory = FutureFactory::new(ledger_services);
    let stream = future_factory.create_stream_from_template(query_id.clone());

    tokio::spawn(
        stream
            .take(1)
            .for_each(move |transaction_id| {
                // TODO: Proceed with sanity checks & Analyze the tx to get vout. See #302
                debug!("Ledger Query Service returned tx: {}", transaction_id);
                //TODO: Mark the trade as failed if cannot deploy the HTLC
                let _ = deploy_eth_htlc(
                    uid,
                    &event_store,
                    &ethereum_service,
                    HtlcId {
                        transaction_id,
                        vout: 0,
                    },
                );

                ledger_query_service_api_client.delete(&query_id);

                Ok(())
            }).map_err(|e| {
                error!("Ledger Query Service Failure: {:#?}", e);
            }),
    );

    json::Response::new(Status::OK(20)).with_body(rfc003::AcceptResponse::<Bitcoin, Ethereum> {
        target_ledger_refund_identity: bob_refund_address,
        source_ledger_success_identity: bob_success_keypair.public_key().into(),
        target_ledger_lock_duration: twelve_hours,
    })
}

fn deploy_eth_htlc<E: EventStore<TradeId>>(
    trade_id: TradeId,
    event_store: &Arc<E>,
    ethereum_service: &Arc<EthereumService>,
    htlc_identifier: HtlcId,
) -> Result<(), Error> {
    let trade_funded: TradeFunded<Ethereum, Bitcoin> = TradeFunded::new(trade_id, htlc_identifier);

    event_store.add_event(trade_id, trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id)?;

    let tx_id = ethereum_service.deploy_htlc(EtherHtlcParams {
        refund_address: order_taken.bob_refund_address,
        success_address: order_taken.alice_success_address,
        time_lock: order_taken.bob_contract_time_lock,
        amount: order_taken.buy_amount,
        secret_hash: order_taken.contract_secret_lock.clone(),
    })?;

    let deployed: ContractDeployed<Ethereum, Bitcoin> =
        ContractDeployed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;
    Ok(())
}
