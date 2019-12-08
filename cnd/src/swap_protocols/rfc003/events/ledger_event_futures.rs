use crate::swap_protocols::{
    asset::Asset,
    rfc003::{
        events::{
            Deployed, DeployedFuture, Funded, FundedFuture, HtlcEvents, RedeemedOrRefundedFuture,
        },
        state_machine::HtlcParams,
        Ledger,
    },
};

// This is an adaptor struct that exists because our current state
// machine implementation requires that we return &mut Futures. This
// is not the actual API we want to implement so we have HtlcEvents
// where the methods return plain Futures and we save them here so we
// don't duplicate them at runtime.
#[allow(missing_debug_implementations)]
pub struct LedgerEventFutures<L: Ledger, A: Asset> {
    htlc_events: Box<dyn HtlcEvents<L, A>>,
    htlc_deployed: Option<Box<DeployedFuture<L>>>,
    htlc_funded: Option<Box<FundedFuture<L, A>>>,
    htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefundedFuture<L>>>,
}

impl<L: Ledger, A: Asset> LedgerEventFutures<L, A> {
    pub fn new(htlc_events: Box<dyn HtlcEvents<L, A>>) -> Self {
        Self {
            htlc_events,
            htlc_deployed: None,
            htlc_funded: None,
            htlc_redeemed_or_refunded: None,
        }
    }

    pub fn htlc_deployed(&mut self, htlc_params: HtlcParams<L, A>) -> &mut DeployedFuture<L> {
        let htlc_events = &self.htlc_events;
        self.htlc_deployed
            .get_or_insert_with(move || htlc_events.htlc_deployed(htlc_params))
    }

    pub fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &Deployed<L>,
    ) -> &mut FundedFuture<L, A> {
        let htlc_events = &self.htlc_events;
        self.htlc_funded
            .get_or_insert_with(move || htlc_events.htlc_funded(htlc_params, htlc_location))
    }

    pub fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_deployment: &Deployed<L>,
        htlc_funding: &Funded<L, A>,
    ) -> &mut RedeemedOrRefundedFuture<L> {
        let htlc_events = &self.htlc_events;
        self.htlc_redeemed_or_refunded.get_or_insert_with(move || {
            htlc_events.htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding)
        })
    }
}
