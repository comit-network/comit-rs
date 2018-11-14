use bitcoin_support::{BitcoinQuantity, Network};
use ethereum_support::{web3::types::Bytes, Address as EthereumAddress, EtherQuantity};
use event_store::EventStore;
use failure::{err_msg, Error};
use futures::{
    sync::mpsc::{self, UnboundedSender},
    Future, Stream,
};
use ledger_query_service::{
    fetch_transaction_stream::FetchTransactionIdStream, BitcoinQuery, EthereumQuery,
    LedgerQueryServiceApiClient,
};
use std::{sync::Arc, time::Duration};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin,
        ethereum::{EtherHtlc, Htlc},
        ledger_htlc_service::{
            BitcoinHtlcFundingParams, BitcoinHtlcRedeemParams, BitcoinService,
            EtherHtlcFundingParams, EtherHtlcRedeemParams, EthereumService, LedgerHtlcService,
        },
    },
};
use swaps::{alice_events::*, common::SwapId};
use tokio_timer::Interval;

#[derive(Debug)]
pub struct AliceLedgerPipeline<
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
    E: EventStore<SwapId>,
> {
    trade_id: SwapId,
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    bitcoin_service: Arc<BitcoinService>,
    bitcoin_network: Network,
    ethereum_service: Arc<EthereumService>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
}

#[derive(Debug)]
pub struct AliceLedgerActor<
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
    E: EventStore<SwapId>,
> {
    event_store: Arc<E>,
    ledger_query_service_api_client: Arc<C>,
    bitcoin_service: Arc<BitcoinService>,
    bitcoin_network: Network,
    ethereum_service: Arc<EthereumService>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
}

impl<C, E> AliceLedgerActor<C, E>
where
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
    E: EventStore<SwapId>,
{
    pub fn new(
        event_store: Arc<E>,
        ledger_query_service_api_client: Arc<C>,
        bitcoin_service: Arc<BitcoinService>,
        bitcoin_network: Network,
        ethereum_service: Arc<EthereumService>,
        bitcoin_poll_interval: Duration,
        ethereum_poll_interval: Duration,
    ) -> Self {
        AliceLedgerActor {
            event_store,
            ledger_query_service_api_client,
            bitcoin_service,
            bitcoin_network,
            ethereum_service,
            bitcoin_poll_interval,
            ethereum_poll_interval,
        }
    }

    pub fn listen(&self) -> (UnboundedSender<SwapId>, impl Future<Item = (), Error = ()>) {
        let (sender, receiver) = mpsc::unbounded();

        let ledger_query_service_api_client = self.ledger_query_service_api_client.clone();
        let event_store = self.event_store.clone();
        let bitcoin_service = self.bitcoin_service.clone();
        let ethereum_service = self.ethereum_service.clone();
        let bitcoin_network = self.bitcoin_network;
        let bitcoin_poll_interval = self.bitcoin_poll_interval;
        let ethereum_poll_interval = self.ethereum_poll_interval;

        let future = receiver
            .for_each(move |trade_id: SwapId| {
                let ledger_query_service_api_client = ledger_query_service_api_client.clone();
                let event_store = event_store.clone();
                let bitcoin_service = bitcoin_service.clone();
                let ethereum_service = ethereum_service.clone();

                let pipeline = AliceLedgerPipeline::new(
                    trade_id,
                    event_store,
                    ledger_query_service_api_client,
                    bitcoin_service,
                    bitcoin_network,
                    ethereum_service,
                    bitcoin_poll_interval,
                    ethereum_poll_interval,
                );

                pipeline
                    .watch_for_btc_funding()
                    .inspect(|pipeline| {
                        let source_funded =
                            SourceFunded::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>::new(
                                pipeline.trade_id,
                            );
                        pipeline
                            .event_store
                            .add_event(pipeline.trade_id, source_funded)
                            .expect("We cannot be in the wrong state");
                    })
                    .and_then(|pipeline| pipeline.watch_eth_deploy())
                    .map(|(pipeline, contract_address)| {
                        let target_funded =
                            TargetFunded::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>::new(
                                contract_address,
                            );
                        pipeline
                            .event_store
                            .add_event(pipeline.trade_id, target_funded)
                            .expect("We cannot be in the wrong state");
                    })
                    .map_err(move |e| {
                        error!(
                            "Halting actions on swap {} because of error: {:?}",
                            trade_id, e
                        )
                    })
            })
            .map_err(|e| {
                panic!("Issue with the unbounded channel: {:?}", e);
            });

        (sender, future)
    }
}

impl<C, E> AliceLedgerPipeline<C, E>
where
    C: LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery>
        + LedgerQueryServiceApiClient<Ethereum, EthereumQuery>,
    E: EventStore<SwapId>,
{
    pub fn new(
        trade_id: SwapId,
        event_store: Arc<E>,
        ledger_query_service_api_client: Arc<C>,
        bitcoin_service: Arc<BitcoinService>,
        bitcoin_network: Network,
        ethereum_service: Arc<EthereumService>,
        bitcoin_poll_interval: Duration,
        ethereum_poll_interval: Duration,
    ) -> Self {
        AliceLedgerPipeline {
            trade_id,
            event_store,
            ledger_query_service_api_client,
            bitcoin_service,
            bitcoin_network,
            ethereum_service,
            bitcoin_poll_interval,
            ethereum_poll_interval,
        }
    }

    pub fn watch_for_btc_funding(self) -> impl Future<Item = Self, Error = Error> {
        let sent_swap_request: SentSwapRequest<
            Bitcoin,
            Ethereum,
            BitcoinQuantity,
            EtherQuantity,
        > = self
            .event_store
            .get_event(self.trade_id)
            .expect("We cannot be in the wrong state");

        let swap_request_accepted: SwapRequestAccepted<
            Bitcoin,
            Ethereum,
            BitcoinQuantity,
            EtherQuantity,
        > = self
            .event_store
            .get_event(self.trade_id)
            .expect("We cannot be in the wrong state");

        let bitcoin_htlc_params = BitcoinHtlcFundingParams {
            refund_pubkey_hash: sent_swap_request.source_ledger_refund_identity,
            success_pubkey_hash: swap_request_accepted.source_ledger_success_identity,
            time_lock: sent_swap_request.source_ledger_lock_duration,
            amount: sent_swap_request.source_asset,
            secret_hash: sent_swap_request.secret.hash(),
        };

        let query = LedgerHtlcService::<
            Bitcoin,
            BitcoinHtlcFundingParams,
            BitcoinHtlcRedeemParams,
            BitcoinQuery,
        >::create_query_to_watch_funding(
            self.bitcoin_service.as_ref(),
            bitcoin_htlc_params.clone(),
        );

        self.ledger_query_service_api_client
            .create_query(query)
            .map_err(Error::from)
            .and_then(|query_id| {
                let stream = self
                    .ledger_query_service_api_client
                    .fetch_transaction_id_stream(
                        Interval::new_interval(self.bitcoin_poll_interval),
                        query_id.clone(),
                    );

                stream
                    .into_future()
                    .map_err(|(e, _)| Error::from(e))
                    .and_then(|(transaction_id, _)| {
                        transaction_id.ok_or_else(|| {
                            err_msg(
                            "LQS stream terminated before it found ethereum deployment transaction",
                        )
                        })
                    })
                    .and_then(move |transaction_id| {
                        debug!("Ledger Query Service returned tx: {}", transaction_id);

                        let bitcoin_htlc_params = bitcoin_htlc_params.clone();

                        let btc_htlc_address = bitcoin::Htlc::new(
                            bitcoin_htlc_params.success_pubkey_hash,
                            bitcoin_htlc_params.refund_pubkey_hash,
                            bitcoin_htlc_params.secret_hash,
                            bitcoin_htlc_params.time_lock.into(),
                        )
                        .compute_address(self.bitcoin_network);

                        let (_n, vout) = self
                            .bitcoin_service
                            .get_vout_matching(&transaction_id, &btc_htlc_address.script_pubkey())
                            .expect("Could not connect to Bitcoin node")
                            .expect("Could not retrieve vout of BTC funding transaction");

                        if vout.value < sent_swap_request.source_asset.satoshi() {
                            return Err(err_msg("Not enough money sent to BTC HTLC, aborting swap"));
                        }

                        self.ledger_query_service_api_client.delete(&query_id);

                        Ok(self)
                    })
            })
    }

    fn watch_eth_deploy(self) -> impl Future<Item = (Self, EthereumAddress), Error = Error> {
        let sent_swap_request: SentSwapRequest<
            Bitcoin,
            Ethereum,
            BitcoinQuantity,
            EtherQuantity,
        > = self
            .event_store
            .get_event(self.trade_id)
            .expect("We cannot be in the wrong state");;

        let swap_request_accepted: SwapRequestAccepted<
            Bitcoin,
            Ethereum,
            BitcoinQuantity,
            EtherQuantity,
        > = self
            .event_store
            .get_event(self.trade_id)
            .expect("We cannot be in the wrong state");;

        let ethereum_htlc_params = EtherHtlcFundingParams {
            refund_address: swap_request_accepted.target_ledger_refund_identity,
            success_address: sent_swap_request.target_ledger_success_identity,
            time_lock: swap_request_accepted.target_ledger_lock_duration,
            amount: sent_swap_request.target_asset,
            secret_hash: sent_swap_request.secret.hash(),
        };

        let query = LedgerHtlcService::<
            Ethereum,
            EtherHtlcFundingParams,
            EtherHtlcRedeemParams,
            EthereumQuery,
        >::create_query_to_watch_funding(
            self.ethereum_service.as_ref(),
            ethereum_htlc_params.clone(),
        );

        self.ledger_query_service_api_client
            .clone()
            .create_query(query)
            .map_err(Error::from)
            .and_then(|query_id| {
                let stream = self
                    .ledger_query_service_api_client
                    .fetch_transaction_id_stream(
                        Interval::new_interval(self.bitcoin_poll_interval),
                        query_id.clone(),
                    );

                stream
                    .into_future()
                    .map_err(|(e, _stream)| Error::from(e))
                    .and_then(move |(transaction_id, _stream)| {
                        let transaction_id = transaction_id.ok_or_else(||err_msg(
                            "LQS stream terminated before it found ethereum deployment transaction",
                        ))?;
                        debug!("Ledger Query Service returned tx: {:?}", transaction_id);

                        let expected_data: Bytes = EtherHtlc::new(
                            ethereum_htlc_params.time_lock,
                            ethereum_htlc_params.refund_address,
                            ethereum_htlc_params.success_address,
                            ethereum_htlc_params.secret_hash,
                        ).compile_to_hex()
                        .into();

                        let transaction =
                            match self.ethereum_service.get_transaction(transaction_id) {
                                Ok(Some(transaction)) => transaction,
                                Ok(None) => {
                                    return Err(format_err!(
                                        "The ETH transaction returned by the Ledger Query Service does not exist, id: {:?}", transaction_id
                                    ))
                                }
                                Err(e) => return Err(format_err!("Issue retrieving ETH deploy transaction: {:?}", e)),
                            };

                        if transaction.input != expected_data {
                            return Err(format_err!("The ETH deployment transaction data does not match expectations, aborting swap.\n Expected: {:?}\nActual  : {:?}", expected_data, transaction.input));
                        }

                        self.ledger_query_service_api_client.delete(&query_id);

                        let contract_address = match self
                            .ethereum_service
                            .get_contract_address(transaction_id)
                            {
                                Ok(Some(contract_address)) => contract_address,
                                Ok(None) => return Err(format_err!("No contract was deployed out of the ETH deployment transaction: {:?}!", transaction)),
                                Err(e) => return Err(format_err!("Issue retrieving ETH deploy transaction receipt: {:?}", e)),
                            };

                        Ok((self, contract_address))
                    })
            })
    }
}
