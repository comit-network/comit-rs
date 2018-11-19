use bitcoin_support::{
    Address as BitcoinAddress, BitcoinQuantity, IntoP2wpkhAddress, Network, OutPoint,
};
use ethereum_support::{web3::types::H256, EtherQuantity, ToEthereumAddress};
use event_store::EventStore;
use failure;
use futures::{
    stream::Stream,
    sync::{mpsc::UnboundedReceiver, oneshot},
    Future,
};
use key_store::KeyStore;
use ledger_query_service::{
    fetch_transaction_stream::FetchTransactionIdStream, BitcoinQuery, EthereumQuery,
    LedgerQueryServiceApiClient,
};
use std::{sync::Arc, time::Duration};
use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    metadata_store::MetadataStore,
    rfc003::{
        self,
        bob::SwapRequestKind,
        ethereum::Seconds,
        ledger_htlc_service::{
            BitcoinHtlcRedeemParams, BitcoinService, EtherHtlcFundingParams, EtherHtlcRedeemParams,
            EthereumService, LedgerHtlcService,
        },
        roles::Bob,
        state_machine::*,
        state_store::StateStore,
        Ledger,
    },
};
use swaps::{
    bob_events::{ContractDeployed, ContractRedeemed, OrderTaken, TradeFunded},
    common::SwapId,
};
use tokio::timer::Interval;

#[derive(Debug)]
pub struct SwapRequestHandler<E, C, MetadataStore, StateStore> {
    // new dependencies
    pub receiver: UnboundedReceiver<(
        SwapId,
        SwapRequestKind,
        oneshot::Sender<Result<rfc003::bob::SwapResponseKind, failure::Error>>,
    )>,
    pub metadata_store: Arc<MetadataStore>,
    pub state_store: Arc<StateStore>,

    // legacy dependencies
    pub event_store: Arc<E>,
    pub lqs_api_client: Arc<C>,
    pub key_store: Arc<KeyStore>,
    pub ethereum_service: Arc<EthereumService>,
    pub bitcoin_service: Arc<BitcoinService>,
    pub bitcoin_poll_interval: Duration,
    pub ethereum_poll_interval: Duration,
}

impl<
        E: EventStore<SwapId>,
        C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
            + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
        M: MetadataStore<SwapId>,
        S: StateStore<SwapId>,
    > SwapRequestHandler<E, C, M, S>
{
    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let (receiver, metadata_store, bitcoin_poll_interval, ethereum_poll_interval) = (
            self.receiver,
            self.metadata_store,
            self.bitcoin_poll_interval,
            self.ethereum_poll_interval,
        );
        let key_store = Arc::clone(&self.key_store);
        let state_store = Arc::clone(&self.state_store);

        let event_store = Arc::clone(&self.event_store);
        let ethereum_service = Arc::clone(&self.ethereum_service);
        let bitcoin_service = Arc::clone(&self.bitcoin_service);
        let lqs_api_client = Arc::clone(&self.lqs_api_client);

        receiver
            .for_each(move |(id, requests, response_sender)| match requests {
                rfc003::bob::SwapRequestKind::BitcoinEthereumBitcoinQuantityEthereumQuantity(
                    request,
                ) => {
                    if let Err(e) = metadata_store.insert(id, request.clone()) {
                        error!("Failed to store metadata for swap {} because {:?}", id, e);

                        // Return Ok to keep the loop running
                        return Ok(());
                    }

                    {
                        let request = request.clone();

                        let start_state = Start {
                            source_ledger_refund_identity: request.source_ledger_refund_identity,
                            target_ledger_success_identity: request.target_ledger_success_identity,
                            source_ledger: request.source_ledger,
                            target_ledger: request.target_ledger,
                            source_asset: request.source_asset,
                            target_asset: request.target_asset,
                            source_ledger_lock_duration: request.source_ledger_lock_duration,
                            secret: request.secret_hash,
                        };

                        spawn_state_machine(id, start_state, state_store.as_ref());
                    }

                    // Legacy code below

                    let network = request.source_ledger.network;

                    let result = process(
                        id,
                        request,
                        &key_store,
                        Arc::clone(&event_store),
                        Arc::clone(&lqs_api_client),
                        Arc::clone(&ethereum_service),
                        Arc::clone(&bitcoin_service),
                        network,
                        bitcoin_poll_interval,
                        ethereum_poll_interval,
                    );

                    response_sender
                        .send(result.map(rfc003::bob::SwapResponseKind::BitcoinEthereum))
                        .map_err(|_| ())
                }
            })
            .map_err(|_| ())
    }
}

fn spawn_state_machine<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: StateStore<SwapId>>(
    id: SwapId,
    start_state: Start<Bob<SL, TL, SA, TA>>,
    state_store: &S,
) {
    let state = SwapStates::Start(start_state);

    // TODO: spawn state machine from state here

    state_store.insert(id, state).expect("handle errors :)");
}

const EXTRA_DATA_FOR_TRANSIENT_REDEEM: [u8; 1] = [1];

fn process<
    E: EventStore<SwapId>,
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
>(
    swap_id: SwapId,
    request: rfc003::bob::SwapRequest<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
    key_store: &KeyStore,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_service: Arc<BitcoinService>,
    bitcoin_network: Network,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
) -> Result<rfc003::bob::SwapResponse<Bitcoin, Ethereum>, failure::Error> {
    let alice_refund_address: BitcoinAddress = {
        use swap_protocols::Ledger;

        request
            .source_ledger
            .address_for_identity(request.source_ledger_refund_identity)
    };

    let bob_success_keypair =
        key_store.get_transient_keypair(&swap_id.into(), &EXTRA_DATA_FOR_TRANSIENT_REDEEM);
    let bob_success_address: BitcoinAddress = bob_success_keypair
        .public_key()
        .into_p2wpkh_address(request.source_ledger.network);
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
        uid: swap_id,
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

    event_store
        .add_event(order_taken.uid, order_taken.clone())
        .unwrap();

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
        .map_err(failure::Error::from)
        .and_then(move |query_id| {
            let stream = ledger_query_service_api_client.fetch_transaction_id_stream(
                Interval::new_interval(bitcoin_poll_interval),
                query_id.clone(),
            );

            stream
                .take(1)
                .map_err(failure::Error::from)
                .for_each(move |transaction_id| {
                    let (n, vout) = bitcoin_service
                        .get_vout_matching(&transaction_id, &htlc_address.script_pubkey())?
                        .ok_or(CounterpartyDeployError::NotFound)?;

                    if vout.value < order_taken.sell_amount.satoshi() {
                        return Err(failure::Error::from(CounterpartyDeployError::Underfunded));
                    }

                    debug!("Ledger Query Service returned tx: {}", transaction_id);
                    let eth_htlc_txid = deploy_eth_htlc(
                        swap_id,
                        event_store.as_ref(),
                        ethereum_service.as_ref(),
                        OutPoint {
                            txid: transaction_id,
                            vout: n as u32,
                        },
                    )?;

                    ledger_query_service_api_client.delete(&query_id);

                    watch_for_eth_htlc_and_redeem_btc_htlc(
                        swap_id,
                        Arc::clone(&ledger_query_service_api_client),
                        eth_htlc_txid,
                        ethereum_poll_interval,
                        Arc::clone(&event_store),
                        Arc::clone(&bitcoin_service),
                        Arc::clone(&ethereum_service),
                    )?;

                    Ok(())
                })
        });

    tokio::spawn(create_query.map_err(|e| {
        error!("Ledger Query Service Failure: {:#?}", e);
    }));

    Ok(rfc003::bob::SwapResponse::Accept {
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
    event_store: &E,
    ethereum_service: &EthereumService,
    htlc_identifier: OutPoint,
) -> Result<H256, failure::Error> {
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
) -> Result<(), failure::Error> {
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
        .map_err(failure::Error::from)
        .and_then(move |query_id| {
            let stream = ledger_query_service_api_client.fetch_transaction_id_stream(
                Interval::new_interval(poll_interval),
                query_id.clone(),
            );

            stream
                .take(1)
                .map_err(failure::Error::from)
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

                    let contract_redeemed: ContractRedeemed<Ethereum, Bitcoin> =
                        ContractRedeemed::new(trade_id, redeem_tx_id.to_string());
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
