use super::AcceptResponse;
use comit_client::SwapResponseError;
use futures::{future::Either, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use swap_protocols::rfc003::{ledger::Ledger, messages::Request};

#[derive(Debug, PartialEq)]
pub enum StateMachineError {
    SwapResponse(SwapResponseError),
}

// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#[allow(type_alias_bounds)]
pub mod events {
    use comit_client::SwapReject;
    use swap_protocols::rfc003::{
        ledger::Ledger, messages::AcceptResponse, state_machine::StateMachineError,
    };
    use tokio::{self, prelude::future::Either};

    type Future<I> = tokio::prelude::future::Future<Item = I, Error = StateMachineError> + Send;

    pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
    pub type Funded<L: Ledger> = Future<L::HtlcId>;
    pub type Refunded<L: Ledger> = Future<L::TxId>;
    pub type Redeemed<L: Ledger> = Future<L::TxId>;
    pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
        Future<Either<(SL::TxId, Box<Funded<TL>>), (TL::HtlcId, Box<Refunded<SL>>)>>;
    pub type RedeemedOrRefunded<L: Ledger> =
        Future<Either<(L::TxId, Box<Redeemed<L>>), (L::TxId, Box<Refunded<L>>)>>;

}

pub trait Services<SL: Ledger, TL: Ledger, SA, TA>: Send {
    fn send_request(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>>;

    fn source_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>>;

    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_id: &TL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>>;

    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_id: &SL::HtlcId,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>>;
}

#[derive(Debug, PartialEq)]
pub enum SwapOutcome {
    Rejected,
    SourceRefunded,
    BothRefunded,
    BothRedeemed,
    SourceRedeemedTargetRefunded,
    SourceRefundedTargetRedeemed,
}

/// TODO: Things to tests:
/// - Side-effects (call to `Services` are only caused if the Option<Future> inside the state is None
///
#[derive(StateMachineFuture)]
pub enum Swap<SL: Ledger, TL: Ledger, SA, TA> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Sent {
        request: Request<SL, TL, SA, TA>,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(
        SourceFundedTargetRefunded,
        SourceRefundedTargetFunded,
        SourceRedeemedTargetFunded,
        SourceFundedTargetRedeemed
    ))]
    BothFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
        source_htlc_id: SL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        request: Request<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_redeemed_txid: TL::TxId,
        source_htlc_id: SL::HtlcId,

        services: Box<Services<SL, TL, SA, TA>>,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(StateMachineError),
}

impl<SL: Ledger, TL: Ledger, SA, TA> Sent<SL, TL, SA, TA> {
    fn inner_future(&mut self) -> &mut Box<events::Response<SL, TL>> {
        self.services.send_request(&self.request)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> Accepted<SL, TL, SA, TA> {
    fn inner_future(&mut self) -> &mut Box<events::Funded<SL>> {
        self.services
            .source_htlc_funded(&self.request, &self.response)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceFunded<SL, TL, SA, TA> {
    fn inner_future(&mut self) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>> {
        self.services.source_htlc_refunded_target_htlc_funded(
            &self.request,
            &self.response,
            &self.source_htlc_id,
        )
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> BothFunded<SL, TL, SA, TA> {
    fn target_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>> {
        self.services
            .target_htlc_redeemed_or_refunded(&self.target_htlc_id)
    }

    fn source_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>> {
        self.services
            .source_htlc_redeemed_or_refunded(&self.source_htlc_id)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceFundedTargetRefunded<SL, TL, SA, TA> {
    fn source_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>> {
        self.services
            .source_htlc_redeemed_or_refunded(&self.source_htlc_id)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceRefundedTargetFunded<SL, TL, SA, TA> {
    fn target_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>> {
        self.services
            .target_htlc_redeemed_or_refunded(&self.target_htlc_id)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceRedeemedTargetFunded<SL, TL, SA, TA> {
    fn target_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>> {
        self.services
            .target_htlc_redeemed_or_refunded(&self.target_htlc_id)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> SourceFundedTargetRedeemed<SL, TL, SA, TA> {
    fn source_htlc_redeemed_or_refunded_future(
        &mut self,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>> {
        self.services
            .source_htlc_redeemed_or_refunded(&self.source_htlc_id)
    }
}

impl<SL: Ledger, TL: Ledger, SA, TA> PollSwap<SL, TL, SA, TA> for Swap<SL, TL, SA, TA> {
    fn poll_sent<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Sent<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSent<SL, TL, SA, TA>>, StateMachineError> {
        let response = try_ready!(state.inner_future().poll());

        let state = state.take();

        match response {
            Ok(swap_accepted) => transition!(Accepted {
                services: state.services,
                request: state.request,
                response: swap_accepted,
            }),
            Err(rejected) => transition!(Final(SwapOutcome::Rejected)),
        }
    }

    fn poll_accepted<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Accepted<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterAccepted<SL, TL, SA, TA>>, StateMachineError> {
        let source_htlc_id = try_ready!(state.inner_future().poll());

        let state = state.take();

        transition!(SourceFunded {
            request: state.request,
            response: state.response,
            services: state.services,
            source_htlc_id,
        })
    }

    fn poll_source_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFunded<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSourceFunded<SL, TL, SA, TA>>, StateMachineError> {
        match try_ready!(state.inner_future().poll()) {
            Either::A((source_refunded_txid, target_htlc_funded_future)) => {
                let state = state.take();
                transition!(Final(SwapOutcome::SourceRefunded))
            }
            Either::B((target_htlc_id, source_htlc_refunded_future)) => {
                let state = state.take();
                transition!(BothFunded {
                    request: state.request,
                    response: state.response,
                    services: state.services,
                    source_htlc_id: state.source_htlc_id,
                    target_htlc_id,
                })
            }
        }
    }

    fn poll_both_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, BothFunded<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterBothFunded<SL, TL, SA, TA>>, StateMachineError> {
        if let Async::Ready(redeemed_or_refunded) =
            state.source_htlc_redeemed_or_refunded_future().poll()?
        {
            let state = state.take();
            match redeemed_or_refunded {
                Either::A((source_redeemed_txid, _)) => transition!(SourceRedeemedTargetFunded {
                    request: state.request,
                    response: state.response,
                    target_htlc_id: state.target_htlc_id,
                    services: state.services,
                }),
                Either::B((source_refunded_txid, _)) => transition!(SourceRefundedTargetFunded {
                    request: state.request,
                    response: state.response,
                    target_htlc_id: state.target_htlc_id,
                    services: state.services,
                }),
            }
        }

        match try_ready!(state.target_htlc_redeemed_or_refunded_future().poll()) {
            Either::A((target_redeemed_txid, _)) => {
                let state = state.take();
                transition!(SourceFundedTargetRedeemed {
                    request: state.request,
                    response: state.response,
                    target_redeemed_txid,
                    source_htlc_id: state.source_htlc_id,
                    services: state.services,
                })
            }
            Either::B((target_refunded_txid, _)) => {
                let state = state.take();
                transition!(SourceFundedTargetRefunded {
                    request: state.request,
                    response: state.response,
                    services: state.services,
                    source_htlc_id: state.source_htlc_id,
                })
            }
        }
    }

    fn poll_source_funded_target_refunded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRefunded<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSourceFundedTargetRefunded>, StateMachineError> {
        match try_ready!(state.source_htlc_redeemed_or_refunded_future().poll()) {
            Either::A((source_redeemed_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRedeemedTargetRefunded))
            }
            Either::B((source_refunded_txid, _)) => transition!(Final(SwapOutcome::BothRefunded)),
        }
    }

    fn poll_source_refunded_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRefundedTargetFunded<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSourceRefundedTargetFunded>, StateMachineError> {
        match try_ready!(state.target_htlc_redeemed_or_refunded_future().poll()) {
            Either::A((target_redeemed_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRefundedTargetRedeemed))
            }
            Either::B((target_refunded_txid, _)) => transition!(Final(SwapOutcome::BothRefunded)),
        }
    }

    fn poll_source_redeemed_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRedeemedTargetFunded<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSourceRedeemedTargetFunded>, StateMachineError> {
        match try_ready!(state.target_htlc_redeemed_or_refunded_future().poll()) {
            Either::A((target_redeemed_txid, _)) => transition!(Final(SwapOutcome::BothRedeemed)),
            Either::B((target_refunded_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRedeemedTargetRefunded))
            }
        }
    }

    fn poll_source_funded_target_redeemed<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRedeemed<SL, TL, SA, TA>>,
    ) -> Result<Async<AfterSourceFundedTargetRedeemed>, StateMachineError> {
        match try_ready!(state.source_htlc_redeemed_or_refunded_future().poll()) {
            Either::A((target_redeemed_txid, _)) => transition!(Final(SwapOutcome::BothRedeemed)),
            Either::B((target_refunded_txid, _)) => {
                transition!(Final(SwapOutcome::SourceRefundedTargetRedeemed))
            }
        }
    }
}
