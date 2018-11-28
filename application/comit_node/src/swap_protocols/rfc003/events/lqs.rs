use ethereum_support::Erc20Quantity;
use futures::{
    future::{self, Either},
    Future,
};
use ledger_query_service::{CreateQuery, EthereumQuery, FirstMatch, Query, QueryIdCache};
use swap_protocols::{
    self,
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        self,
        ethereum::erc20,
        events::{
            Deployed, Funded, LedgerEvents, NewHtlcFundedQuery, NewHtlcRedeemedQuery,
            NewHtlcRefundedQuery, RedeemedOrRefunded,
        },
        find_htlc_location::FindHtlcLocation,
        state_machine::HtlcParams,
        Ledger,
    },
};

#[allow(missing_debug_implementations)]
pub struct LqsEvents<L: Ledger, Q: Query> {
    create_ledger_query: QueryIdCache<L, Q>,
    ledger_first_match: FirstMatch<L>,

    htlc_deployed_and_funded: Option<Box<Deployed<L>>>,
    htlc_funded: Option<Box<Funded<L>>>,
    htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<L>>>,
}

impl<L: Ledger, Q: Query> LqsEvents<L, Q> {
    pub fn new(create_ledger_query: QueryIdCache<L, Q>, ledger_first_match: FirstMatch<L>) -> Self {
        Self {
            create_ledger_query,
            ledger_first_match,
            htlc_deployed_and_funded: None,
            htlc_funded: None,
            htlc_redeemed_or_refunded: None,
        }
    }

    fn htlc_deployed<A>(&mut self, htlc_params: HtlcParams<L, A>, query: Q) -> &mut Deployed<L>
    where
        A: Asset,
        <L as swap_protocols::Ledger>::Transaction: FindHtlcLocation<L, A>,
    {
        let ledger_first_match = self.ledger_first_match.clone();
        let query_id = self.create_ledger_query.create_query(query);

        self.htlc_deployed_and_funded.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    ledger_first_match
                        .first_match_of(query_id)
                        .and_then(move |tx| {
                            tx.find_htlc_location(&htlc_params)
                                .map_err(|_| rfc003::Error::InsufficientFunding)
                        })
                });

            Box::new(funded_future)
        })
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        refunded_query: Q,
        redeemed_query: Q,
    ) -> &mut RedeemedOrRefunded<L> {
        let ledger_first_match = self.ledger_first_match.clone();
        let refunded_query_id = self.create_ledger_query.create_query(refunded_query);
        let redeemed_query_id = self.create_ledger_query.create_query(redeemed_query);

        self.htlc_redeemed_or_refunded.get_or_insert_with(move || {
            let inner_first_match = ledger_first_match.clone();
            let refunded_future = refunded_query_id
                .map_err(rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| inner_first_match.first_match_of(query_id));
            let inner_first_match = ledger_first_match.clone();
            let redeemed_future = redeemed_query_id
                .map_err(rfc003::Error::LedgerQueryService)
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
impl<L, A, Q> LedgerEvents<L, A> for LqsEvents<L, Q>
where
    L: Ledger,
    A: Asset,
    Q: Query + NewHtlcRefundedQuery<L, A> + NewHtlcFundedQuery<L, A> + NewHtlcRedeemedQuery<L, A>,
    <L as swap_protocols::Ledger>::Transaction: FindHtlcLocation<L, A>,
{
    fn htlc_deployed(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Deployed<L> {
        let query = Q::new_htlc_funded_query(&htlc_params);
        self.htlc_deployed(htlc_params, query)
    }

    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<L, A>,
        _htlc_location: &L::HtlcLocation,
    ) -> &mut Funded<L> {
        self.htlc_funded.get_or_insert(Box::new(future::ok(None)))
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L> {
        let refunded_query = Q::new_htlc_refunded_query(&htlc_params, htlc_location);
        let redeemed_query = Q::new_htlc_redeemed_query(&htlc_params, htlc_location);

        self.htlc_redeemed_or_refunded(refunded_query, redeemed_query)
    }
}

#[allow(missing_debug_implementations)]
pub struct LqsEventsForErc20 {
    lqs_events: LqsEvents<Ethereum, EthereumQuery>,
}

impl LqsEventsForErc20 {
    pub fn new(
        create_ledger_query: QueryIdCache<Ethereum, EthereumQuery>,
        ledger_first_match: FirstMatch<Ethereum>,
    ) -> Self {
        Self {
            lqs_events: LqsEvents {
                create_ledger_query,
                ledger_first_match,
                htlc_deployed_and_funded: None,
                htlc_funded: None,
                htlc_redeemed_or_refunded: None,
            },
        }
    }
}

impl LedgerEvents<Ethereum, Erc20Quantity> for LqsEventsForErc20 {
    fn htlc_deployed(
        &mut self,
        htlc_params: HtlcParams<Ethereum, Erc20Quantity>,
    ) -> &mut Deployed<Ethereum> {
        let query = erc20::new_htlc_deployed_query(&htlc_params);
        self.lqs_events.htlc_deployed(htlc_params, query)
    }

    fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<Ethereum, Erc20Quantity>,
        htlc_location: &<Ethereum as Ledger>::HtlcLocation,
    ) -> &mut Funded<Ethereum> {
        let query = erc20::new_htlc_funded_query(&htlc_params, htlc_location);
        let query_id = self.lqs_events.create_ledger_query.create_query(query);

        let ledger_first_match = self.lqs_events.ledger_first_match.clone();
        self.lqs_events.htlc_funded.get_or_insert_with(move || {
            let funded_future = query_id
                .map_err(rfc003::Error::LedgerQueryService)
                .and_then(move |query_id| {
                    ledger_first_match
                        .first_match_of(query_id)
                        .map(move |tx| Some(tx))
                });

            Box::new(funded_future)
        })
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        _htlc_params: HtlcParams<Ethereum, Erc20Quantity>,
        htlc_location: &<Ethereum as Ledger>::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<Ethereum> {
        let refunded_query = erc20::new_htlc_refunded_query(htlc_location);
        let redeemed_query = erc20::new_htlc_redeemed_query(htlc_location);

        self.lqs_events
            .htlc_redeemed_or_refunded(refunded_query, redeemed_query)
    }
}
