use crate::{
    btsieve::{CreateQuery, EthereumQuery, FirstMatch, Query, QueryIdCache},
    swap_protocols::{
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
            secret::SecretHash,
            state_machine::HtlcParams,
            ExtractSecret, FundTransaction, Ledger, RedeemTransaction, RefundTransaction,
        },
    },
};
use ethereum_support::Erc20Token;
use futures::{
    future::{self, Either},
    Future,
};

#[allow(missing_debug_implementations)]
pub struct BtsieveEvents<L: Ledger, Q: Query> {
    create_ledger_query: QueryIdCache<L, Q>,
    ledger_first_match: FirstMatch<L>,

    htlc_deployed_and_funded: Option<Box<Deployed<L>>>,
    htlc_funded: Option<Box<Funded<L>>>,
    htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<L>>>,
}

impl<L: Ledger, Q: Query> BtsieveEvents<L, Q>
where
    L::Transaction: ExtractSecret,
{
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
            let funded_future =
                query_id
                    .map_err(rfc003::Error::Btsieve)
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
        redeemed_query: Q,
        refunded_query: Q,
        secret_hash: SecretHash,
    ) -> &mut RedeemedOrRefunded<L> {
        let ledger_first_match = self.ledger_first_match.clone();
        let redeemed_query_id = self.create_ledger_query.create_query(redeemed_query);
        let refunded_query_id = self.create_ledger_query.create_query(refunded_query);

        self.htlc_redeemed_or_refunded.get_or_insert_with(move || {
            let inner_first_match = ledger_first_match.clone();
            let redeemed_future = redeemed_query_id
                .map_err(rfc003::Error::Btsieve)
                .and_then(move |query_id| inner_first_match.first_match_of(query_id))
                .and_then(move |transaction| {
                    let secret = transaction.extract_secret(&secret_hash).ok_or_else(|| {
                        error!(
                            "Redeem transaction didn't have secret it in: {:?}",
                            transaction
                        );
                        rfc003::Error::Internal(
                            "Redeem transaction didn't have the secret in it".into(),
                        )
                    })?;
                    Ok(RedeemTransaction {
                        transaction,
                        secret,
                    })
                });
            let inner_first_match = ledger_first_match.clone();
            let refunded_future = refunded_query_id
                .map_err(rfc003::Error::Btsieve)
                .and_then(move |query_id| inner_first_match.first_match_of(query_id))
                .map(RefundTransaction);

            Box::new(
                redeemed_future
                    .select2(refunded_future)
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
impl<L, A, Q> LedgerEvents<L, A> for BtsieveEvents<L, Q>
where
    L: Ledger,
    L::Transaction: ExtractSecret,
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

        self.htlc_redeemed_or_refunded(redeemed_query, refunded_query, htlc_params.secret_hash)
    }
}

#[allow(missing_debug_implementations)]
pub struct BtsieveEventsForErc20 {
    btsieve_events: BtsieveEvents<Ethereum, EthereumQuery>,
}

impl BtsieveEventsForErc20 {
    pub fn new(
        create_ledger_query: QueryIdCache<Ethereum, EthereumQuery>,
        ledger_first_match: FirstMatch<Ethereum>,
    ) -> Self {
        Self {
            btsieve_events: BtsieveEvents {
                create_ledger_query,
                ledger_first_match,
                htlc_deployed_and_funded: None,
                htlc_funded: None,
                htlc_redeemed_or_refunded: None,
            },
        }
    }
}

impl LedgerEvents<Ethereum, Erc20Token> for BtsieveEventsForErc20 {
    fn htlc_deployed(
        &mut self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
    ) -> &mut Deployed<Ethereum> {
        let query = erc20::new_htlc_deployed_query(&htlc_params);
        self.btsieve_events.htlc_deployed(htlc_params, query)
    }

    fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
        htlc_location: &<Ethereum as Ledger>::HtlcLocation,
    ) -> &mut Funded<Ethereum> {
        let query = erc20::new_htlc_funded_query(&htlc_params, htlc_location);
        let query_id = self.btsieve_events.create_ledger_query.create_query(query);

        let ledger_first_match = self.btsieve_events.ledger_first_match.clone();
        self.btsieve_events.htlc_funded.get_or_insert_with(move || {
            let funded_future =
                query_id
                    .map_err(rfc003::Error::Btsieve)
                    .and_then(move |query_id| {
                        ledger_first_match
                            .first_match_of(query_id)
                            .map(FundTransaction)
                            .map(Some)
                    });

            Box::new(funded_future)
        })
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
        htlc_location: &<Ethereum as Ledger>::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<Ethereum> {
        let refunded_query = erc20::new_htlc_refunded_query(htlc_location);
        let redeemed_query = erc20::new_htlc_redeemed_query(htlc_location);

        self.btsieve_events.htlc_redeemed_or_refunded(
            redeemed_query,
            refunded_query,
            htlc_params.secret_hash,
        )
    }
}
