use comit_client::Client;
use futures::{future::Either, Future};
use ledger_query_service::{CreateQuery, FirstMatch, Query, QueryIdCache};
use std::sync::Arc;
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
        IntoSecretHash, Ledger,
    },
};

#[derive(Debug)]
pub enum Player<ComitClient> {
    Alice { client: Arc<ComitClient> },
    Bob,
}

#[allow(missing_debug_implementations)]
pub struct DefaultEvents<SL: Ledger, TL: Ledger, ComitClient, SLQuery: Query, TLQuery: Query> {
    player: Player<ComitClient>,

    create_source_ledger_query: QueryIdCache<SL, SLQuery>,
    source_ledger_first_match: FirstMatch<SL>,

    create_target_ledger_query: QueryIdCache<TL, TLQuery>,
    target_ledger_first_match: FirstMatch<TL>,

    response: Option<Box<Response<SL, TL>>>,
    source_htlc_funded_query: Option<Box<Funded<SL>>>,
    source_htlc_refunded_target_htlc_funded_query:
        Option<Box<SourceRefundedOrTargetFunded<SL, TL>>>,
    source_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<SL>>>,
    target_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<TL>>>,
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
    SA: Asset + IsContainedInSourceLedgerTransaction<SL, TL, TA, S>,
    TA: Asset,
    S: IntoSecretHash,
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
        let source_ledger_first_match = self.source_ledger_first_match.clone();

        let query = SLQuery::new_source_htlc_funded_query(&swap);
        let query_id = self.create_source_ledger_query.create_query(query);

        self.source_htlc_funded_query.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    source_ledger_first_match
                        .first_match_of(query_id)
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
    TA: Asset + IsContainedInTargetLedgerTransaction<SL, TL, SA, S>,
    S: IntoSecretHash,
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

        let source_ledger_first_match = self.source_ledger_first_match.clone();
        let source_refunded_query =
            SLQuery::new_source_htlc_refunded_query(&swap, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);

        let target_ledger_first_match = self.target_ledger_first_match.clone();
        let target_funded_query = TLQuery::new_target_htlc_funded_query(&swap);
        let target_funded_query_id = self
            .create_target_ledger_query
            .create_query(target_funded_query);

        self.source_htlc_refunded_target_htlc_funded_query
            .get_or_insert_with(move || {
                let source_refunded_future = source_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| source_ledger_first_match.first_match_of(query_id));

                let target_funded_future = target_funded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        target_ledger_first_match
                            .first_match_of(query_id)
                            .and_then(move |tx| {
                                TA::is_contained_in_target_ledger_transaction(swap, tx)
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
    S: IntoSecretHash,
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

        let target_ledger_first_match = self.target_ledger_first_match.clone();
        let target_refunded_query =
            TLQuery::new_target_htlc_refunded_query(&swap, target_htlc_location);
        let target_refunded_query_id = self
            .create_target_ledger_query
            .create_query(target_refunded_query);

        let target_redeemed_query =
            TLQuery::new_target_htlc_redeemed_query(&swap, target_htlc_location);
        let target_redeemed_query_id = self
            .create_target_ledger_query
            .create_query(target_redeemed_query);

        self.target_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_first_match = target_ledger_first_match.clone();
                let target_refunded_future = target_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));
                let inner_first_match = target_ledger_first_match.clone();
                let target_redeemed_future = target_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));

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
    S: IntoSecretHash,
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

        let source_ledger_first_match = self.source_ledger_first_match.clone();
        let source_refunded_query =
            SLQuery::new_source_htlc_refunded_query(&swap, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);

        let source_redeemed_query =
            SLQuery::new_source_htlc_redeemed_query(&swap, source_htlc_location);
        let source_redeemed_query_id = self
            .create_source_ledger_query
            .create_query(source_redeemed_query);

        self.source_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_first_match = source_ledger_first_match.clone();
                let source_refunded_future = source_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));
                let inner_first_match = source_ledger_first_match.clone();
                let source_redeemed_future = source_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));

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
    SA: Asset + IsContainedInSourceLedgerTransaction<SL, TL, TA, S>,
    TA: Asset + IsContainedInTargetLedgerTransaction<SL, TL, SA, S>,
    S: IntoSecretHash,
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
