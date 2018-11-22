use futures::{future::Either, Future};
use ledger_query_service::{CreateQuery, FirstMatch, Query, QueryIdCache};
use swap_protocols::{
    asset::Asset,
    rfc003::{
        self,
        events::{
            Funded, LedgerEvents, NewHtlcFundedQuery, NewHtlcRedeemedQuery, NewHtlcRefundedQuery,
            RedeemedOrRefunded,
        },
        is_contained_in_transaction::IsContainedInTransaction,
        state_machine::HtlcParams,
        Ledger,
    },
};

#[allow(missing_debug_implementations)]
pub struct LqsEvents<L: Ledger, Q: Query> {
    create_ledger_query: QueryIdCache<L, Q>,
    ledger_first_match: FirstMatch<L>,

    htlc_funded_query: Option<Box<Funded<L>>>,
    htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<L>>>,
}

impl<L, A, Q> LedgerEvents<L, A> for LqsEvents<L, Q>
where
    L: Ledger,
    A: Asset + IsContainedInTransaction<L>,
    Q: Query + NewHtlcRefundedQuery<L, A> + NewHtlcFundedQuery<L, A> + NewHtlcRedeemedQuery<L, A>,
{
    fn htlc_funded(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Funded<L> {
        let ledger_first_match = self.ledger_first_match.clone();

        let query = Q::new_htlc_funded_query(&htlc_params);
        let query_id = self.create_ledger_query.create_query(query);

        self.htlc_funded_query.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    ledger_first_match
                        .first_match_of(query_id)
                        .and_then(move |tx| {
                            A::is_contained_in_transaction(&htlc_params, tx)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L> {
        let ledger_first_match = self.ledger_first_match.clone();
        let refunded_query = Q::new_htlc_refunded_query(&htlc_params, htlc_location);
        let refunded_query_id = self.create_ledger_query.create_query(refunded_query);

        let redeemed_query = Q::new_htlc_redeemed_query(&htlc_params, htlc_location);
        let redeemed_query_id = self.create_ledger_query.create_query(redeemed_query);

        self.htlc_redeemed_or_refunded.get_or_insert_with(move || {
            let inner_first_match = ledger_first_match.clone();
            let refunded_future = refunded_query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| inner_first_match.first_match_of(query_id));
            let inner_first_match = ledger_first_match.clone();
            let redeemed_future = redeemed_query_id
                .map_err(|_| rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| inner_first_match.first_match_of(query_id));

            Box::new(
                refunded_future
                    .select2(redeemed_future)
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
