use crate::swap_protocols::{
    asset::Asset,
    rfc003::{
        events::{
            DeployTransaction, Deployed, Funded, HtlcEvents, LedgerEvents, RedeemedOrRefunded,
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
    htlc_deployed: Option<Box<Deployed<L>>>,
    htlc_funded: Option<Box<Funded<L, A>>>,
    htlc_redeemed_or_refunded: Option<Box<RedeemedOrRefunded<L>>>,
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
}

impl<L: Ledger, A: Asset> LedgerEvents<L, A> for LedgerEventFutures<L, A> {
    fn htlc_deployed(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Deployed<L> {
        let htlc_events = &self.htlc_events;
        self.htlc_deployed
            .get_or_insert_with(move || htlc_events.htlc_deployed(htlc_params))
    }

    fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &DeployTransaction<L>,
    ) -> &mut Funded<L, A> {
        let htlc_events = &self.htlc_events;
        self.htlc_funded
            .get_or_insert_with(move || htlc_events.htlc_funded(htlc_params, htlc_location))
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L> {
        let htlc_events = &self.htlc_events;
        self.htlc_redeemed_or_refunded.get_or_insert_with(move || {
            htlc_events.htlc_redeemed_or_refunded(htlc_params, htlc_location)
        })
    }
}
