use bam::{
    config::Config,
    json::{self, Request, Response},
    Status,
};
use bitcoin_support::{
    Address as BitcoinAddress, BitcoinQuantity, IntoP2wpkhAddress, Network, OutPoint,
};
use ethereum_support::{web3::types::H256, EtherQuantity, ToEthereumAddress};
use event_store::EventStore;
use failure::Error;
use futures::{future, Future, Stream};
use key_store::KeyStore;
use ledger_query_service::{
    fetch_transaction_stream::FetchTransactionIdStream, BitcoinQuery, EthereumQuery,
    LedgerQueryServiceApiClient,
};
use std::{sync::Arc, time::Duration};
use swap_protocols::{
    bam_types::{SwapProtocol, SwapRequestHeaders, SwapResponse},
    handler::SwapRequestHandler,
    ledger::{Bitcoin, Ethereum, Ledger},
    rfc003::{
        self,
        ethereum::Seconds,
        ledger_htlc_service::{
            BitcoinHtlcRedeemParams, BitcoinService, EtherHtlcFundingParams, EtherHtlcRedeemParams,
            EthereumService, LedgerHtlcService,
        },
    },
};
use swaps::{
    bob_events::{
        ContractDeployed, ContractRedeemed as BobContractRedeemed, OrderTaken, TradeFunded,
    },
    common::SwapId,
};
use tokio;
use tokio_timer::Interval;

pub fn json_config<
    H: SwapRequestHandler<rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>>
        + SwapRequestHandler<rfc003::Request<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>>,
    E: EventStore<SwapId>,
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
>(
    mut handler: H,
    key_store: Arc<KeyStore>,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_service: Arc<BitcoinService>,
    bitcoin_network: Network,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
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
            use swap_protocols::bam_types::{Asset, Ledger};

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
                        let response = match handler.handle(request.clone()) {
                            SwapResponse::Decline => Response::new(Status::SE(21)),
                            SwapResponse::Accept => process(
                                request,
                                &key_store,
                                event_store.clone(),
                                ledger_query_service_api_client.clone(),
                                ethereum_service.clone(),
                                bitcoin_service.clone(),
                                bitcoin_network,
                                bitcoin_poll_interval,
                                ethereum_poll_interval,
                            ),
                        };

                        Box::new(future::ok(response))
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
                        let response = match handler.handle(request.clone()) {
                            SwapResponse::Decline => Response::new(Status::SE(21)),
                            SwapResponse::Accept => Response::new(Status::SE(22)),
                        };

                        Box::new(future::ok(response))
                    }
                    _ => Box::new(future::ok(Response::new(Status::SE(22)))), // 22 = unsupported pair or source/target combination
                },
            }
        },
    )
}

const EXTRA_DATA_FOR_TRANSIENT_REDEEM: [u8; 1] = [1];

fn process<
    E: EventStore<SwapId>,
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
>(
    request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    key_store: &Arc<KeyStore>,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_service: Arc<BitcoinService>,
    bitcoin_network: Network,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
) -> Response {
    let alice_refund_address: BitcoinAddress = request
        .source_ledger
        .address_for_identity(request.source_ledger_refund_identity);

    let trade_id = SwapId::default();

    let bob_success_keypair =
        key_store.get_transient_keypair(&trade_id.into(), &EXTRA_DATA_FOR_TRANSIENT_REDEEM);
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

    let twelve_hours = Seconds(60 * 60 * 12);

    let order_taken = OrderTaken::<Ethereum, Bitcoin> {
        uid: trade_id,
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

    let htlc = rfc003::bitcoin::Htlc::new(
        bob_success_address,
        alice_refund_address,
        request.secret_hash,
        btc_lock_time,
    );

    let htlc_address = htlc.compute_address(bitcoin_network);

    let query = BitcoinQuery::Transaction {
        to_address: Some(htlc_address.clone()),
        from_outpoint: None,
        unlock_script: None,
    };

    let create_query = ledger_query_service_api_client
        .create_query(query)
        .map_err(Error::from)
        .and_then(move |query_id| {
            let stream = ledger_query_service_api_client.fetch_transaction_id_stream(
                Interval::new_interval(bitcoin_poll_interval),
                query_id.clone(),
            );

            stream
                .take(1)
                .map_err(Error::from)
                .for_each(move |transaction_id| {
                    let (n, vout) = bitcoin_service
                        .get_vout_matching(&transaction_id, &htlc_address.script_pubkey())?
                        .ok_or(CounterpartyDeployError::NotFound)?;

                    if vout.value < order_taken.sell_amount.satoshi() {
                        return Err(Error::from(CounterpartyDeployError::Underfunded));
                    }

                    debug!("Ledger Query Service returned tx: {}", transaction_id);
                    let eth_htlc_txid = deploy_eth_htlc(
                        trade_id,
                        &event_store,
                        &ethereum_service,
                        OutPoint {
                            txid: transaction_id,
                            vout: n as u32,
                        },
                    )?;

                    ledger_query_service_api_client.delete(&query_id);

                    watch_for_eth_htlc_and_redeem_btc_htlc(
                        trade_id,
                        ledger_query_service_api_client.clone(),
                        eth_htlc_txid,
                        ethereum_poll_interval,
                        event_store.clone(),
                        bitcoin_service.clone(),
                        ethereum_service.clone(),
                    )?;

                    Ok(())
                })
        });

    tokio::spawn(create_query.map_err(|e| {
        error!("Ledger Query Service Failure: {:#?}", e);
    }));

    json::Response::new(Status::OK(20)).with_body(rfc003::AcceptResponseBody::<Bitcoin, Ethereum> {
        target_ledger_refund_identity: bob_refund_address,
        source_ledger_success_identity: bob_success_keypair.public_key().into(),
        target_ledger_lock_duration: twelve_hours,
    })
}

#[derive(Debug, Fail)]
enum CounterpartyDeployError {
    #[fail(display = "The contract was funded but it was less than the agreed amount")]
    Underfunded,
    #[fail(display = "The contract wasn't found at the id provided")]
    NotFound,
}

fn deploy_eth_htlc<E: EventStore<SwapId>>(
    trade_id: SwapId,
    event_store: &Arc<E>,
    ethereum_service: &Arc<EthereumService>,
    htlc_identifier: OutPoint,
) -> Result<H256, Error> {
    let trade_funded: TradeFunded<Ethereum, Bitcoin> = TradeFunded::new(trade_id, htlc_identifier);

    event_store.add_event(trade_id, trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id)?;

    let tx_id = ethereum_service.fund_htlc(EtherHtlcFundingParams {
        refund_address: order_taken.bob_refund_address,
        success_address: order_taken.alice_success_address,
        time_lock: order_taken.bob_contract_time_lock,
        amount: order_taken.buy_amount,
        secret_hash: order_taken.contract_secret_lock.clone(),
    })?;

    let deployed: ContractDeployed<Ethereum, Bitcoin> =
        ContractDeployed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;
    Ok(tx_id)
}

fn watch_for_eth_htlc_and_redeem_btc_htlc<
    C: LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
    E: EventStore<SwapId>,
>(
    trade_id: SwapId,
    ledger_query_service_api_client: Arc<C>,
    eth_htlc_created_tx_id: H256,
    poll_interval: Duration,
    event_store: Arc<E>,
    bitcoin_service: Arc<BitcoinService>,
    ethereum_service: Arc<EthereumService>,
) -> Result<(), Error> {
    let query = LedgerHtlcService::<
        Ethereum,
        EtherHtlcFundingParams,
        EtherHtlcRedeemParams,
        EthereumQuery,
    >::create_query_to_watch_redeeming(
        ethereum_service.as_ref(), eth_htlc_created_tx_id
    )?;

    let create_query = ledger_query_service_api_client
        .create_query(query)
        .map_err(Error::from)
        .and_then(move |query_id| {
            let stream = ledger_query_service_api_client.fetch_transaction_id_stream(
                Interval::new_interval(poll_interval),
                query_id.clone(),
            );

            stream
                .take(1)
                .map_err(Error::from)
                .for_each(move |transaction_id| {
                    debug!(
                        "Ledger Query Service returned tx sent to Ethereum HTLC: {}",
                        transaction_id
                    );

                    let secret = LedgerHtlcService::<
                        Ethereum,
                        EtherHtlcFundingParams,
                        EtherHtlcRedeemParams,
                        EthereumQuery,
                    >::check_and_extract_secret(
                        ethereum_service.as_ref(),
                        eth_htlc_created_tx_id,
                        transaction_id,
                    )?;

                    let order_taken: OrderTaken<Ethereum, Bitcoin> =
                        event_store.get_event(trade_id)?;

                    let trade_funded: TradeFunded<Ethereum, Bitcoin> =
                        event_store.get_event(trade_id)?;

                    let htlc_redeem_params = BitcoinHtlcRedeemParams {
                        htlc_identifier: trade_funded.htlc_identifier,
                        success_address: order_taken.bob_success_address,
                        refund_address: order_taken.alice_refund_address,
                        amount: order_taken.sell_amount,
                        time_lock: order_taken.alice_contract_time_lock,
                        keypair: order_taken.bob_success_keypair,
                        secret,
                    };

                    let redeem_tx_id = bitcoin_service.redeem_htlc(trade_id, htlc_redeem_params)?;

                    let contract_redeemed: BobContractRedeemed<
                        Ethereum,
                        Bitcoin,
                    > = BobContractRedeemed::new(trade_id, redeem_tx_id.to_string());
                    event_store.add_event(trade_id, contract_redeemed)?;

                    ledger_query_service_api_client.delete(&query_id);

                    Ok(())
                })
        });

    tokio::spawn(create_query.map_err(|e| {
        error!("Ledger Query Service Failure: {:#?}", e);
    }));
    Ok(())
}
