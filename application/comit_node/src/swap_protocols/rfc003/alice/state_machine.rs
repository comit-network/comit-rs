#![allow(missing_debug_implementations)]
use comit_client::SwapResponseError;
use failure;
use futures::{future::Either, Async, Future};
use ledger_query_service;
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::sync::{Arc, RwLock};
use swap_protocols::rfc003::{ledger::Ledger, messages::Request, secret::Secret, AcceptResponse};

#[derive(Debug, Clone, PartialEq)]
pub enum StateMachineError {
    SwapResponse(SwapResponseError),
    LedgerQueryService(String),
    TimerError,
    InsufficientFunding,
}

impl From<ledger_query_service::Error> for StateMachineError {
    fn from(e: ledger_query_service::Error) -> Self {
        StateMachineError::LedgerQueryService(failure::Error::from(e).to_string())
    }
}

// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#[allow(type_alias_bounds)]
pub mod events {
    use super::StateMachineError;
    use comit_client::SwapReject;
    use swap_protocols::rfc003::{ledger::Ledger, messages::AcceptResponse};
    use tokio::{self, prelude::future::Either};

    type Future<I> = tokio::prelude::future::Future<Item = I, Error = StateMachineError> + Send;

    pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
    pub type Funded<L: Ledger> = Future<L::HtlcId>;
    pub type Refunded<L: Ledger> = Future<L::TxId>;
    pub type Redeemed<L: Ledger> = Future<L::TxId>;
    pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
        Future<Either<SL::TxId, TL::HtlcId>>;
    pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::TxId, L::TxId>>;

}

impl<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> SwapFuture<SL, TL, SA, TA> {
    pub fn new(
        initial_state: SwapStates<SL, TL, SA, TA>,
        context: Context<SL, TL, SA, TA>,
    ) -> Self {
        SwapFuture(Some(initial_state), context)
    }
}

pub trait Futures<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone>: Send {
    fn send_request(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>>;

    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA>,
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

pub struct Context<SL: Ledger, TL: Ledger, SA, TA> {
    pub futures: Box<Futures<SL, TL, SA, TA>>,
    pub state_repo: Arc<SaveState<SL, TL, SA, TA>>,
}

macro_rules! transition_save {
    ( $repo:expr, $new_state:expr) => {{
        let save_state = $new_state;
        $repo.save(save_state.clone().into());
        return Ok(::futures::Async::Ready(save_state.into()));
    }};
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome {
    Rejected,
    SourceRefunded,
    BothRefunded,
    BothRedeemed,
    SourceRedeemedTargetRefunded,
    SourceRefundedTargetRedeemed,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
pub enum Swap<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        source_identity: SL::HtlcIdentity,
        target_identity: TL::HtlcIdentity,
        source_ledger: SL,
        target_ledger: TL,
        source_asset: SA,
        target_asset: TA,
        source_ledger_lock_duration: SL::LockDuration,
        secret: Secret,
    },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(
        SourceFundedTargetRefunded,
        SourceRefundedTargetFunded,
        SourceRedeemedTargetFunded,
        SourceFundedTargetRedeemed
    ))]
    BothFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcId,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        start: Start<SL, TL, SA, TA>,
        response: AcceptResponse<SL, TL>,
        target_redeemed_txid: TL::TxId,
        source_htlc_id: SL::HtlcId,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(StateMachineError),
}

impl<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone> PollSwap<SL, TL, SA, TA>
    for Swap<SL, TL, SA, TA>
{
    fn poll_start<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Start<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterStart<SL, TL, SA, TA>>, StateMachineError> {
        let request = Request {
            source_asset: state.source_asset.clone(),
            target_asset: state.target_asset.clone(),
            source_ledger: state.source_ledger.clone(),
            target_ledger: state.target_ledger.clone(),
            source_ledger_refund_identity: state.source_identity.clone().into(),
            target_ledger_success_identity: state.target_identity.clone().into(),
            source_ledger_lock_duration: state.source_ledger_lock_duration.clone(),
            secret_hash: state.secret.hash(),
        };

        let response = try_ready!(context.futures.send_request(&request).poll());

        let state = state.take();

        match response {
            Ok(swap_accepted) => transition_save!(
                context.state_repo,
                Accepted {
                    start: state,
                    response: swap_accepted,
                }
            ),
            Err(_) => transition_save!(context.state_repo, Final(SwapOutcome::Rejected)),
        }
    }

    fn poll_accepted<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Accepted<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterAccepted<SL, TL, SA, TA>>, StateMachineError> {
        let source_htlc_id = try_ready!(
            context
                .futures
                .source_htlc_funded(&state.start, &state.response)
                .poll()
        );

        let state = state.take();

        transition_save!(
            context.state_repo,
            SourceFunded {
                start: state.start,
                response: state.response,
                source_htlc_id,
            }
        )
    }

    fn poll_source_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFunded<SL, TL, SA, TA>>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_refunded_target_htlc_funded(
                    &state.start,
                    &state.response,
                    &state.source_htlc_id
                ).poll()
        ) {
            Either::A(_source_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::SourceRefunded))
            }
            Either::B(target_htlc_id) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    BothFunded {
                        start: state.start,
                        response: state.response,
                        source_htlc_id: state.source_htlc_id,
                        target_htlc_id,
                    }
                )
            }
        }
    }

    fn poll_both_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, BothFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterBothFunded<SL, TL, SA, TA>>, StateMachineError> {
        if let Async::Ready(redeemed_or_refunded) = context
            .futures
            .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
            .poll()?
        {
            let state = state.take();
            match redeemed_or_refunded {
                Either::A(_source_redeemed_txid) => transition_save!(
                    context.state_repo,
                    SourceRedeemedTargetFunded {
                        start: state.start,
                        response: state.response,
                        target_htlc_id: state.target_htlc_id,
                    }
                ),
                Either::B(_source_refunded_txid) => transition_save!(
                    context.state_repo,
                    SourceRefundedTargetFunded {
                        start: state.start,
                        response: state.response,
                        target_htlc_id: state.target_htlc_id,
                    }
                ),
            }
        }

        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A(target_redeemed_txid) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    SourceFundedTargetRedeemed {
                        start: state.start,
                        response: state.response,
                        target_redeemed_txid,
                        source_htlc_id: state.source_htlc_id,
                    }
                )
            }
            Either::B(_target_refunded_txid) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    SourceFundedTargetRefunded {
                        start: state.start,
                        response: state.response,
                        source_htlc_id: state.source_htlc_id,
                    }
                )
            }
        }
    }

    fn poll_source_funded_target_refunded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRefunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFundedTargetRefunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
                .poll()
        ) {
            Either::A(_source_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRedeemedTargetRefunded)
            ),
            Either::B(_source_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_source_refunded_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRefundedTargetFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceRefundedTargetFunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A(_target_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRefundedTargetRedeemed)
            ),
            Either::B(_target_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_source_redeemed_target_funded<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRedeemedTargetFunded<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceRedeemedTargetFunded>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .target_htlc_redeemed_or_refunded(&state.target_htlc_id)
                .poll()
        ) {
            Either::A(_target_redeemed_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRedeemed))
            }
            Either::B(_target_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRedeemedTargetRefunded)
            ),
        }
    }

    fn poll_source_funded_target_redeemed<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRedeemed<SL, TL, SA, TA>>,
        context: &mut Context<SL, TL, SA, TA>,
    ) -> Result<Async<AfterSourceFundedTargetRedeemed>, StateMachineError> {
        match try_ready!(
            context
                .futures
                .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
                .poll()
        ) {
            Either::A(_target_redeemed_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRedeemed))
            }
            Either::B(_target_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRefundedTargetRedeemed)
            ),
        }
    }
}

pub trait SaveState<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone>: Send + Sync {
    fn save(&self, state: SwapStates<SL, TL, SA, TA>);
}

impl<SL: Ledger, TL: Ledger, SA: Clone + Send + Sync, TA: Clone + Send + Sync>
    SaveState<SL, TL, SA, TA> for RwLock<SwapStates<SL, TL, SA, TA>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA>) {
        let _self = &mut *self.write().unwrap();
        *_self = state;
    }
}

use futures::sync::mpsc;

impl<SL: Ledger, TL: Ledger, SA: Clone + Send + Sync, TA: Clone + Send + Sync>
    SaveState<SL, TL, SA, TA> for mpsc::UnboundedSender<SwapStates<SL, TL, SA, TA>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA>) {
        // ignore error the subscriber is no longer interested in state updates
        let _ = self.unbounded_send(state);
    }
}
