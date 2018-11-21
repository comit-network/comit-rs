use futures::{future::Either, Future};
use ledger_query_service::{CreateQuery, FirstMatch, Query, QueryIdCache};
use swap_protocols::{
    asset::Asset,
    rfc003::{
        self,
        events::{
            AlphaRefundedOrBetaFunded, Funded, LedgerEvents, NewHtlcFundedQuery,
            NewHtlcRedeemedQuery, NewHtlcRefundedQuery, RedeemedOrRefunded,
        },
        is_contained_in_transaction::IsContainedInTransaction,
        state_machine::HtlcParams,
        Ledger,
    },
};

#[allow(missing_debug_implementations)]
pub struct LqsEvents<AL: Ledger, BL: Ledger, ALQuery: Query, BLQuery: Query> {
    create_alpha_ledger_query: QueryIdCache<AL, ALQuery>,
    alpha_ledger_first_match: FirstMatch<AL>,

    create_beta_ledger_query: QueryIdCache<BL, BLQuery>,
    beta_ledger_first_match: FirstMatch<BL>,

    alpha_htlc_funded_query: Option<Box<Funded<AL>>>,
    alpha_htlc_refunded_beta_htlc_funded_query: Option<Box<AlphaRefundedOrBetaFunded<AL, BL>>>,
    alpha_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<AL>>>,
    beta_htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<BL>>>,
}

impl<AL, BL, AA, BA, ALQuery, BLQuery> LedgerEvents<AL, BL, AA, BA>
    for LqsEvents<AL, BL, ALQuery, BLQuery>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset + IsContainedInTransaction<AL>,
    BA: Asset + IsContainedInTransaction<BL>,
    ALQuery: Query
        + NewHtlcRefundedQuery<AL, AA>
        + NewHtlcFundedQuery<AL, AA>
        + NewHtlcRedeemedQuery<AL, AA>,
    BLQuery: Query
        + NewHtlcRefundedQuery<BL, BA>
        + NewHtlcFundedQuery<BL, BA>
        + NewHtlcRedeemedQuery<BL, BA>,
{
    fn alpha_htlc_funded(&mut self, htlc_params: HtlcParams<AL, AA>) -> &mut Funded<AL> {
        let alpha_ledger_first_match = self.alpha_ledger_first_match.clone();

        let query = ALQuery::new_htlc_funded_query(&htlc_params);
        let query_id = self.create_alpha_ledger_query.create_query(query);

        self.alpha_htlc_funded_query.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    alpha_ledger_first_match
                        .first_match_of(query_id)
                        .and_then(move |tx| {
                            AA::is_contained_in_transaction(&htlc_params, tx)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }

    fn alpha_htlc_refunded_beta_htlc_funded(
        &mut self,
        alpha_htlc_params: HtlcParams<AL, AA>,
        beta_htlc_params: HtlcParams<BL, BA>,
        alpha_htlc_location: &AL::HtlcLocation,
    ) -> &mut AlphaRefundedOrBetaFunded<AL, BL> {
        let alpha_ledger_first_match = self.alpha_ledger_first_match.clone();
        let alpha_refunded_query =
            ALQuery::new_htlc_refunded_query(&alpha_htlc_params, alpha_htlc_location);
        let alpha_refunded_query_id = self
            .create_alpha_ledger_query
            .create_query(alpha_refunded_query);

        let beta_ledger_first_match = self.beta_ledger_first_match.clone();
        let beta_funded_query = BLQuery::new_htlc_funded_query(&beta_htlc_params);
        let beta_funded_query_id = self
            .create_beta_ledger_query
            .create_query(beta_funded_query);

        self.alpha_htlc_refunded_beta_htlc_funded_query
            .get_or_insert_with(move || {
                let alpha_refunded_future = alpha_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| alpha_ledger_first_match.first_match_of(query_id));

                let beta_funded_future = beta_funded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| {
                        beta_ledger_first_match
                            .first_match_of(query_id)
                            .and_then(move |tx| {
                                BA::is_contained_in_transaction(&beta_htlc_params, tx)
                                    .map_err(|_| rfc003::Error::InsufficientFunding)
                            })
                    });

                Box::new(
                    alpha_refunded_future
                        .select2(beta_funded_future)
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

    fn beta_htlc_redeemed_or_refunded(
        &mut self,
        beta_htlc_params: HtlcParams<BL, BA>,
        beta_htlc_location: &BL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<BL> {
        let beta_ledger_first_match = self.beta_ledger_first_match.clone();
        let beta_refunded_query =
            BLQuery::new_htlc_refunded_query(&beta_htlc_params, beta_htlc_location);
        let beta_refunded_query_id = self
            .create_beta_ledger_query
            .create_query(beta_refunded_query);

        let beta_redeemed_query =
            BLQuery::new_htlc_redeemed_query(&beta_htlc_params, beta_htlc_location);
        let beta_redeemed_query_id = self
            .create_beta_ledger_query
            .create_query(beta_redeemed_query);

        self.beta_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_first_match = beta_ledger_first_match.clone();
                let beta_refunded_future = beta_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));
                let inner_first_match = beta_ledger_first_match.clone();
                let beta_redeemed_future = beta_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));

                Box::new(
                    beta_refunded_future
                        .select2(beta_redeemed_future)
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

    fn alpha_htlc_redeemed_or_refunded(
        &mut self,
        alpha_htlc_params: HtlcParams<AL, AA>,
        alpha_htlc_location: &AL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<AL> {
        let alpha_ledger_first_match = self.alpha_ledger_first_match.clone();
        let alpha_refunded_query =
            ALQuery::new_htlc_refunded_query(&alpha_htlc_params, alpha_htlc_location);
        let alpha_refunded_query_id = self
            .create_alpha_ledger_query
            .create_query(alpha_refunded_query);

        let alpha_redeemed_query =
            ALQuery::new_htlc_redeemed_query(&alpha_htlc_params, alpha_htlc_location);
        let alpha_redeemed_query_id = self
            .create_alpha_ledger_query
            .create_query(alpha_redeemed_query);

        self.alpha_htlc_redeemed_or_refunded
            .get_or_insert_with(move || {
                let inner_first_match = alpha_ledger_first_match.clone();
                let alpha_refunded_future = alpha_refunded_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));
                let inner_first_match = alpha_ledger_first_match.clone();
                let alpha_redeemed_future = alpha_redeemed_query_id
                    .map_err(|_| rfc003::Error::LedgerQueryService)
                    .and_then(move |query_id| inner_first_match.first_match_of(query_id));

                Box::new(
                    alpha_refunded_future
                        .select2(alpha_redeemed_future)
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
