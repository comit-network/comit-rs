use futures::{future::Either, Future};
use ledger_query_service::{CreateQuery, FirstMatch, Query, QueryIdCache};
use swap_protocols::{
    asset::Asset,
    rfc003::{
        self,
        events::{
            Funded, HtlcFunded, LedgerEvents, NewHtlcFundedQuery, NewHtlcRedeemedQuery,
            NewHtlcRefundedQuery, RedeemedOrRefunded, SourceHtlcRedeemedOrRefunded,
            SourceHtlcRefundedTargetHtlcFunded, SourceRefundedOrTargetFunded,
            TargetHtlcRedeemedOrRefunded,
        },
        is_contained_in_transaction::IsContainedInTransaction,
        state_machine::HtlcParams,
        Ledger,
    },
};

#[allow(missing_debug_implementations)]
pub struct LqsEvents<SL: Ledger, TL: Ledger, SLQuery: Query, TLQuery: Query> {
    create_source_ledger_query: QueryIdCache<SL, SLQuery>,
    source_ledger_first_match: FirstMatch<SL>,

    create_target_ledger_query: QueryIdCache<TL, TLQuery>,
    target_ledger_first_match: FirstMatch<TL>,

    source_htlc_funded_query: Option<Box<Funded<SL>>>,
    source_htlc_refunded_target_htlc_funded_query:
        Option<Box<SourceRefundedOrTargetFunded<SL, TL>>>,
    source_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<SL>>>,
    target_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<TL>>>,
}

impl<SL, TL, SA, SLQuery, TLQuery> HtlcFunded<SL, SA> for LqsEvents<SL, TL, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset + IsContainedInTransaction<SL>,
    SLQuery: Query + NewHtlcFundedQuery<SL, SA>,
    TLQuery: Query,
{
    fn htlc_funded<'s>(&'s mut self, htlc_params: HtlcParams<SL, SA>) -> &'s mut Funded<SL> {
        let source_ledger_first_match = self.source_ledger_first_match.clone();

        let query = SLQuery::new_htlc_funded_query(&htlc_params);
        let query_id = self.create_source_ledger_query.create_query(query);

        self.source_htlc_funded_query.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    source_ledger_first_match
                        .first_match_of(query_id)
                        .and_then(move |tx| {
                            SA::is_contained_in_transaction(&htlc_params, tx)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }
}

impl<SL, TL, SA, TA, SLQuery, TLQuery> SourceHtlcRefundedTargetHtlcFunded<SL, TL, SA, TA>
    for LqsEvents<SL, TL, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset + IsContainedInTransaction<TL>,
    SLQuery: Query + NewHtlcRefundedQuery<SL, SA>,
    TLQuery: Query + NewHtlcFundedQuery<TL, TA>,
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        source_htlc_params: HtlcParams<SL, SA>,
        target_htlc_params: HtlcParams<TL, TA>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut SourceRefundedOrTargetFunded<SL, TL> {
        let source_ledger_first_match = self.source_ledger_first_match.clone();
        let source_refunded_query =
            SLQuery::new_htlc_refunded_query(&source_htlc_params, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);

        let target_ledger_first_match = self.target_ledger_first_match.clone();
        let target_funded_query = TLQuery::new_htlc_funded_query(&target_htlc_params);
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
                                TA::is_contained_in_transaction(&target_htlc_params, tx)
                                    .map_err(|_| rfc003::Error::InsufficientFunding)
                            })
                    });

                Box::new(
                    source_refunded_future
                        .select2(target_funded_future)
                        .map(|either| match either {
                            Either::A((item, _stream)) => Either::A(item),
                            Either::B((item, _stream)) => Either::B(item),
                        })
                        .map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, TA, SLQuery, TLQuery> TargetHtlcRedeemedOrRefunded<TL, TA>
    for LqsEvents<SL, TL, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    TA: Asset,
    TLQuery: Query + NewHtlcRefundedQuery<TL, TA> + NewHtlcRedeemedQuery<TL, TA>,
    SLQuery: Query,
{
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_params: HtlcParams<TL, TA>,
        target_htlc_location: &TL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<TL> {
        let target_ledger_first_match = self.target_ledger_first_match.clone();
        let target_refunded_query =
            TLQuery::new_htlc_refunded_query(&target_htlc_params, target_htlc_location);
        let target_refunded_query_id = self
            .create_target_ledger_query
            .create_query(target_refunded_query);

        let target_redeemed_query =
            TLQuery::new_htlc_redeemed_query(&target_htlc_params, target_htlc_location);
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
                        })
                        .map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, SA, SLQuery, TLQuery> SourceHtlcRedeemedOrRefunded<SL, SA>
    for LqsEvents<SL, TL, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    SLQuery: Query + NewHtlcRefundedQuery<SL, SA> + NewHtlcRedeemedQuery<SL, SA>,
    TLQuery: Query,
{
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_params: HtlcParams<SL, SA>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<SL> {
        let source_ledger_first_match = self.source_ledger_first_match.clone();
        let source_refunded_query =
            SLQuery::new_htlc_refunded_query(&source_htlc_params, source_htlc_location);
        let source_refunded_query_id = self
            .create_source_ledger_query
            .create_query(source_refunded_query);

        let source_redeemed_query =
            SLQuery::new_htlc_redeemed_query(&source_htlc_params, source_htlc_location);
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
                        })
                        .map_err(|either| match either {
                            Either::A((error, _stream)) => error,
                            Either::B((error, _stream)) => error,
                        }),
                )
            })
    }
}

impl<SL, TL, SA, TA, SLQuery, TLQuery> LedgerEvents<SL, TL, SA, TA>
    for LqsEvents<SL, TL, SLQuery, TLQuery>
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset + IsContainedInTransaction<SL>,
    TA: Asset + IsContainedInTransaction<TL>,
    SLQuery: Query
        + NewHtlcFundedQuery<SL, SA>
        + NewHtlcRefundedQuery<SL, SA>
        + NewHtlcRedeemedQuery<SL, SA>,
    TLQuery: Query
        + NewHtlcFundedQuery<TL, TA>
        + NewHtlcRefundedQuery<TL, TA>
        + NewHtlcRedeemedQuery<TL, TA>,
{
}
