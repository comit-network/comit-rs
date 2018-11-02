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
        events::{
            Funded, NewSourceHtlcFundedQuery, NewSourceHtlcRedeemedQuery,
            NewSourceHtlcRefundedQuery, NewTargetHtlcFundedQuery, NewTargetHtlcRedeemedQuery,
            NewTargetHtlcRefundedQuery, RequestResponded, Response, SourceHtlcFunded,
        },
        messages::Request,
        state_machine::OngoingSwap,
        validation::IsContainedInTransaction,
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

    create_source_ledger_query: QueryIdCache<SL, SLQuery>,
    source_ledger_fetch_query_results: Arc<FetchQueryResults<SL>>,
    source_ledger_tick_interval: Duration,

    _create_target_ledger_query: QueryIdCache<TL, TLQuery>,
    _target_ledger_fetch_query_results: Arc<FetchQueryResults<TL>>,
    _target_ledger_tick_interval: Duration,
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
    SA: Asset + IsContainedInTransaction<SL, TL, SA, TA, S>,
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

        let source_asset = swap.source_asset.clone();
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
                        .map(|(txid, _stream)| txid.expect("ticker stream should never terminate"))
                        .map_err(|(e, _stream)| rfc003::Error::LedgerQueryService)
                        .and_then(move |tx_id| {
                            SA::is_contained_in_transaction(swap, &tx_id)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }
}
