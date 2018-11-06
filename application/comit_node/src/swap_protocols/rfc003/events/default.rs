use comit_client::Client;
use futures::{future::Either, stream::Stream, Future};
use ledger_query_service::{
    fetch_transaction_stream::FetchTransactionStream, CreateQuery, FetchFullQueryResults, Query,
    QueryIdCache,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use swap_protocols::{
    asset::Asset,
    rfc003::{
        self,
        events::{
            Events, Funded, NewSourceHtlcFundedQuery, NewSourceHtlcRedeemedQuery,
            NewSourceHtlcRefundedQuery, NewTargetHtlcFundedQuery, NewTargetHtlcRedeemedQuery,
            NewTargetHtlcRefundedQuery, RedeemedOrRefunded, RequestResponded, Response,
            SourceHtlcFunded, SourceHtlcRedeemedOrRefunded, SourceHtlcRefundedTargetHtlcFunded,
            SourceRefundedOrTargetFunded, TargetHtlcRedeemedOrRefunded,
        },
        messages::Request,
        state_machine::OngoingSwap,
        validation::{IsContainedInSourceLedgerTransaction, IsContainedInTargetLedgerTransaction},
        Ledger, SecretHash,
    },
};
use tokio::timer::Interval;

#[derive(Debug)]
pub enum Player<ComitClient> {
    Alice { client: Arc<ComitClient> },
    Bob,
}

#[allow(missing_debug_implementations)]
pub struct DefaultEvents<SL: Ledger, TL: Ledger, ComitClient, SLQuery: Query, TLQuery: Query> {
    player: Player<ComitClient>,
    response: Option<Box<Response<SL, TL>>>,
    source_htlc_funded_query: Option<Box<Funded<SL>>>,
    source_htlc_refunded_target_htlc_funded_query:
        Option<Box<SourceRefundedOrTargetFunded<SL, TL>>>,
    source_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<SL>>>,
    target_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<TL>>>,

    create_source_ledger_query: QueryIdCache<SL, SLQuery>,
    source_ledger_fetch_query_results: Arc<FetchFullQueryResults<SL>>,
    source_ledger_tick_interval: Duration,

    create_target_ledger_query: QueryIdCache<TL, TLQuery>,
    target_ledger_fetch_query_results: Arc<FetchFullQueryResults<TL>>,
    target_ledger_tick_interval: Duration,
}

impl<SL, TL, SA, TA, ComitClient, SLQuery, TLQuery> RequestResponded<SL, TL, SA, TA>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    ComitClient: Client,
    SLQuery: Query,
    TLQuery: Query,
{
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<Response<SL, TL>> {
        match self.player {
            Player::Alice {ref client}=> {
                let client = client.clone();

                self.response.get_or_insert_with(|| {
                    Box::new(
                        client
                            .send_swap_request(request.clone())
                            .map_err(rfc003::Error::SwapResponse),
                    )
                })
            },
            Player::Bob => {
                unimplemented!("return a future that resolves once the user sent a response to the COMIT node via the API")
            }
        }
    }
}

impl<SL, TL, SA, TA, S, ComitClient, SLQuery, TLQuery> SourceHtlcFunded<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset + IsContainedInSourceLedgerTransaction<SL, TL, SA, TA, S>,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone + 'static,
    ComitClient: Client,
    SLQuery: Query
        + NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>,
    TLQuery: Query
        + NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>,
{
    fn source_htlc_funded<'s>(
        &'s mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
    ) -> &'s mut Box<Funded<SL>> {
        let swap = swap.clone();
        let source_ledger_fetch_query_results = self.source_ledger_fetch_query_results.clone();
        let source_ledger_tick_interval = self.source_ledger_tick_interval;

        let query = SLQuery::new_source_htlc_funded_query(&swap);
        let query_id = self.create_source_ledger_query.create_query(query);

        self.source_htlc_funded_query.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    source_ledger_fetch_query_results
                        .fetch_transaction_stream(
                            Interval::new(Instant::now(), source_ledger_tick_interval),
                            query_id,
                        ).take(1)
                        .into_future()
                        .map(|(tx, _stream)| tx.expect("ticker stream should never terminate"))
                        .map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                        .and_then(move |tx| {
                            SA::is_contained_in_source_ledger_transaction(swap, tx)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }
}

impl<SL, TL, SA, TA, S, ComitClient, SLQuery, TLQuery>
    SourceHtlcRefundedTargetHtlcFunded<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset + IsContainedInTargetLedgerTransaction<SL, TL, SA, TA, S>,
    S: Into<SecretHash> + Send + Sync + Clone + 'static,
    ComitClient: Client,
    SLQuery: Query
        + NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>,
    TLQuery: Query
        + NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>,
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut Box<SourceRefundedOrTargetFunded<SL, TL>> {
        let swap = swap.clone();

        let source_ledger_fetch_query_results = self.source_ledger_fetch_query_results.clone();
        let source_refunded_query =
            SLQuery::new_source_htlc_refunded_query(&swap, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);
        let source_ledger_tick_interval = self.source_ledger_tick_interval;

        let target_ledger_fetch_query_results = self.target_ledger_fetch_query_results.clone();
        let target_funded_query = TLQuery::new_target_htlc_funded_query(&swap);
        let target_funded_query_id = self
            .create_target_ledger_query
            .create_query(target_funded_query);
        let target_ledger_tick_interval = self.target_ledger_tick_interval;

        self.source_htlc_refunded_target_htlc_funded_query
            .get_or_insert_with(move || {
                let source_refunded_future = source_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        source_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), source_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                    });

                let target_funded_future = target_funded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        target_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), target_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                            .and_then(move |tx_id| {
                                TA::is_contained_in_target_ledger_transaction(swap, tx_id)
                                    .map_err(|_| rfc003::Error::InsufficientFunding)
                            })
                    });

                Box::new(
                    source_refunded_future
                        .select2(target_funded_future)
                        .map(|either| match either {
                            Either::A((item, _stream)) => Either::A(item),
                            Either::B((item, _stream)) => Either::B(item),
                        }).map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, SA, TA, S, ComitClient, SLQuery, TLQuery>
    TargetHtlcRedeemedOrRefunded<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone + 'static,
    ComitClient: Client,
    SLQuery: Query
        + NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>,
    TLQuery: Query
        + NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>,
{
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
        target_htlc_location: &TL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<TL>> {
        let swap = swap.clone();

        let target_ledger_fetch_query_results = self.target_ledger_fetch_query_results.clone();
        let target_refunded_query =
            TLQuery::new_target_htlc_refunded_query(&swap, target_htlc_location);
        let target_refunded_query_id = self
            .create_target_ledger_query
            .create_query(target_refunded_query);

        let target_ledger_tick_interval = self.target_ledger_tick_interval;
        let target_redeemed_query =
            TLQuery::new_target_htlc_redeemed_query(&swap, target_htlc_location);
        let target_redeemed_query_id = self
            .create_target_ledger_query
            .create_query(target_redeemed_query);

        self.target_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_target_ledger_fetch_query_results =
                    target_ledger_fetch_query_results.clone();
                let target_refunded_future = target_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        inner_target_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), target_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                    });
                let inner_target_ledger_fetch_query_results =
                    target_ledger_fetch_query_results.clone();
                let target_redeemed_future = target_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        inner_target_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), target_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                    });

                Box::new(
                    target_refunded_future
                        .select2(target_redeemed_future)
                        .map(|either| match either {
                            Either::A((item, _stream)) => Either::A(item),
                            Either::B((item, _stream)) => Either::B(item),
                        }).map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, SA, TA, S, ComitClient, SLQuery, TLQuery>
    SourceHtlcRedeemedOrRefunded<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone + 'static,
    ComitClient: Client,
    SLQuery: Query
        + NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>,
    TLQuery: Query
        + NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>,
{
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<SL>> {
        let swap = swap.clone();

        let source_ledger_fetch_query_results = self.source_ledger_fetch_query_results.clone();
        let source_refunded_query =
            SLQuery::new_source_htlc_refunded_query(&swap, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);

        let source_ledger_tick_interval = self.source_ledger_tick_interval;
        let source_redeemed_query =
            SLQuery::new_source_htlc_redeemed_query(&swap, source_htlc_location);
        let source_redeemed_query_id = self
            .create_source_ledger_query
            .create_query(source_redeemed_query);

        self.source_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_source_ledger_fetch_query_results =
                    source_ledger_fetch_query_results.clone();
                let source_refunded_future = source_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        inner_source_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), source_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                    });
                let inner_source_ledger_fetch_query_results =
                    source_ledger_fetch_query_results.clone();
                let source_redeemed_future = source_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        inner_source_ledger_fetch_query_results
                            .fetch_transaction_stream(
                                Interval::new(Instant::now(), source_ledger_tick_interval),
                                query_id,
                            ).take(1)
                            .into_future()
                            .map(|(txid, _stream)| {
                                txid.expect("ticker stream should never terminate")
                            }).map_err(|(_, _stream)| rfc003::Error::LedgerQueryService)
                    });

                Box::new(
                    source_refunded_future
                        .select2(source_redeemed_future)
                        .map(|either| match either {
                            Either::A((item, _stream)) => Either::A(item),
                            Either::B((item, _stream)) => Either::B(item),
                        }).map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, SA, TA, S, ComitClient, SLQuery, TLQuery> Events<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, ComitClient, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset + IsContainedInSourceLedgerTransaction<SL, TL, SA, TA, S>,
    TA: Asset + IsContainedInTargetLedgerTransaction<SL, TL, SA, TA, S>,
    S: Into<SecretHash> + Send + Sync + Clone + 'static,
    ComitClient: Client,
    SLQuery: Query
        + NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>,
    TLQuery: Query
        + NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>
        + NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>,
{}
