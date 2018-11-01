use bitcoin_support::BitcoinQuantity;
use comit_client::Client;
use futures::{stream::Stream, Async, Future};
use ledger_query_service::{
    self, fetch_transaction_stream::FetchTransactionStream, BitcoinQuery, CreateQuery,
    FetchQueryResults, LedgerQueryServiceApiClient, Query, QueryId, QueryIdCache,
};
use std::{
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};
use swap_protocols::{
    asset::Asset,
    ledger::Bitcoin,
    rfc003::{
        self,
        alice::bitcoin_htlc_address,
        events::{Funded, QueryFactory, RequestResponded, Response, SourceHtlcFunded},
        messages::{AcceptResponse, Request},
        state_machine::{OngoingSwap, Start},
        Ledger, SecretHash,
    },
};
use tokio::timer::Interval;

pub enum Player {
    Alice,
    Bob,
}

#[allow(missing_debug_implementations)]
pub struct DefaultEvents<
    SL: Ledger,
    TL: Ledger,
    SA,
    TA,
    S,
    COMIT_CLIENT,
    SL_FETCH_QUERY_RESULTS,
    SL_HFQ: Query,
    SL_CHFQ,
> {
    player: Player,
    response: Option<Box<Response<SL, TL>>>,
    source_htlc_funded_query: Option<Box<Funded<SL>>>,
    source_ledger_tick_interval: Duration,
    client: Arc<COMIT_CLIENT>,

    source_htlc_funded_query_factory: Arc<QueryFactory<SL, TL, SA, TA, S, SL_HFQ>>,
    create_source_htlc_funded_query: QueryIdCache<SL, SL_HFQ, SL_CHFQ>,
    source_ledger_fetch_query_results: Arc<SL_FETCH_QUERY_RESULTS>,

    source_asset_type: PhantomData<SA>,
    target_asset_type: PhantomData<TA>,
    secret_type: PhantomData<S>,
}

impl<SL, TL, SA, TA, S, C, SL_FETCH_QUERY_RESULTS, SL_HFQ, SL_CHFQ> RequestResponded<SL, TL, SA, TA>
    for DefaultEvents<SL, TL, SA, TA, S, C, SL_FETCH_QUERY_RESULTS, SL_HFQ, SL_CHFQ>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    C: Client,
    SL_HFQ: Query,
    S: Into<SecretHash> + Clone + Send + Sync,
    SL_CHFQ: CreateQuery<SL, SL_HFQ>,
    SL_FETCH_QUERY_RESULTS: FetchQueryResults<SL>,
{
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<Response<SL, TL>> {
        let client = self.client.clone();

        self.response.get_or_insert_with(|| {
            Box::new(
                client
                    .send_swap_request(request.clone())
                    .map_err(rfc003::Error::SwapResponse),
            )
        })
    }
}

impl<SL, TL, SA, TA, S, C, SL_FETCH_QUERY_RESULTS, SL_HFQ, SL_CHFQ>
    SourceHtlcFunded<SL, TL, SA, TA, S>
    for DefaultEvents<SL, TL, SA, TA, S, C, SL_FETCH_QUERY_RESULTS, SL_HFQ, SL_CHFQ>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    C: Client,
    S: Into<SecretHash> + Send + Sync + Clone,
    SL_HFQ: Query,
    SL_CHFQ: CreateQuery<SL, SL_HFQ>,
    SL_FETCH_QUERY_RESULTS: FetchQueryResults<SL>,
{
    fn source_htlc_funded<'s>(
        &'s mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
    ) -> &'s mut Box<Funded<SL>> {
        let source_ledger_fetch_query_results = self.source_ledger_fetch_query_results.clone();

        let source_asset = swap.source_asset.clone();
        let source_ledger_tick_interval = self.source_ledger_tick_interval;

        let query = self.source_htlc_funded_query_factory.create(swap);
        let query_id = self.create_source_htlc_funded_query.create_query(query);

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
                        .map(|(txid, _stream)| txid.expect("ticker stream should never terminate"))
                        .map_err(|(e, _stream)| rfc003::Error::LedgerQueryService)
                        .and_then(move |tx_id| Ok(unimplemented!("validate tx here")))
                });

            Box::new(funded_future)
        })
    }
}
