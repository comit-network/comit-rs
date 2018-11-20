use comit_client;
use futures::{future::Either, Async};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::sync::Arc;
use swap_protocols::rfc003::ExtractSecret;

use swap_protocols::{
    self,
    asset::Asset,
    rfc003::{
        self, events, ledger::Ledger, roles::Role, SaveState, Secret, SecretHash, SwapOutcome,
    },
};

#[derive(Debug, Clone)]
pub struct StateMachineResponse<SLSI, TLRI, TLLD> {
    pub source_ledger_success_identity: SLSI,
    pub target_ledger_refund_identity: TLRI,
    pub target_ledger_lock_duration: TLLD,
}

impl<SL: Ledger, TL: Ledger> From<comit_client::rfc003::AcceptResponseBody<SL, TL>>
    for StateMachineResponse<SL::Identity, TL::Identity, TL::LockDuration>
{
    fn from(accept_response: comit_client::rfc003::AcceptResponseBody<SL, TL>) -> Self {
        Self {
            source_ledger_success_identity: accept_response.source_ledger_success_identity,
            target_ledger_refund_identity: accept_response.target_ledger_refund_identity,
            target_ledger_lock_duration: accept_response.target_ledger_lock_duration,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HtlcParams<L: Ledger, A: Asset> {
    pub asset: A,
    pub ledger: L,
    pub success_identity: L::Identity,
    pub refund_identity: L::Identity,
    pub lock_duration: L::LockDuration,
    pub secret_hash: SecretHash,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OngoingSwap<R: Role> {
    pub source_ledger: R::SourceLedger,
    pub target_ledger: R::TargetLedger,
    pub source_asset: R::SourceAsset,
    pub target_asset: R::TargetAsset,
    pub source_ledger_success_identity: R::SourceSuccessHtlcIdentity,
    pub source_ledger_refund_identity: R::SourceRefundHtlcIdentity,
    pub target_ledger_success_identity: R::TargetSuccessHtlcIdentity,
    pub target_ledger_refund_identity: R::TargetRefundHtlcIdentity,
    pub source_ledger_lock_duration: <R::SourceLedger as Ledger>::LockDuration,
    pub target_ledger_lock_duration: <R::TargetLedger as Ledger>::LockDuration,
    pub secret: R::Secret,
}

impl<R: Role> OngoingSwap<R> {
    pub fn new(
        start: Start<R>,
        response: StateMachineResponse<
            R::SourceSuccessHtlcIdentity,
            R::TargetRefundHtlcIdentity,
            <R::TargetLedger as Ledger>::LockDuration,
        >,
    ) -> Self {
        OngoingSwap {
            source_ledger: start.source_ledger,
            target_ledger: start.target_ledger,
            source_asset: start.source_asset,
            target_asset: start.target_asset,
            source_ledger_success_identity: response.source_ledger_success_identity,
            source_ledger_refund_identity: start.source_ledger_refund_identity,
            target_ledger_success_identity: start.target_ledger_success_identity,
            target_ledger_refund_identity: response.target_ledger_refund_identity,
            source_ledger_lock_duration: start.source_ledger_lock_duration,
            target_ledger_lock_duration: response.target_ledger_lock_duration,
            secret: start.secret,
        }
    }

    pub fn source_htlc_params(&self) -> HtlcParams<R::SourceLedger, R::SourceAsset> {
        HtlcParams {
            asset: self.source_asset.clone(),
            ledger: self.source_ledger.clone(),
            success_identity: self.source_ledger_success_identity.clone().into(),
            refund_identity: self.source_ledger_refund_identity.clone().into(),
            lock_duration: self.source_ledger_lock_duration.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }

    pub fn target_htlc_params(&self) -> HtlcParams<R::TargetLedger, R::TargetAsset> {
        HtlcParams {
            asset: self.target_asset.clone(),
            ledger: self.target_ledger.clone(),
            success_identity: self.target_ledger_success_identity.clone().into(),
            refund_identity: self.target_ledger_refund_identity.clone().into(),
            lock_duration: self.target_ledger_lock_duration.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct Context<R: Role> {
    pub ledger_events:
        Box<events::LedgerEvents<R::SourceLedger, R::TargetLedger, R::SourceAsset, R::TargetAsset>>,
    pub state_repo: Arc<SaveState<R>>,
    pub response_event: Box<events::CommunicationEvents<R> + Send>,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
#[allow(missing_debug_implementations, clippy::too_many_arguments)]
pub enum Swap<R: Role> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        source_ledger_refund_identity: R::SourceRefundHtlcIdentity,
        target_ledger_success_identity: R::TargetSuccessHtlcIdentity,
        source_ledger: R::SourceLedger,
        target_ledger: R::TargetLedger,
        source_asset: R::SourceAsset,
        target_asset: R::TargetAsset,
        source_ledger_lock_duration: <R::SourceLedger as Ledger>::LockDuration,
        secret: R::Secret,
    },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted { swap: OngoingSwap<R> },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        swap: OngoingSwap<R>,
        source_htlc_location: <R::SourceLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(
        SourceFundedTargetRedeemed,
        SourceFundedTargetRefunded,
        SourceRefundedTargetFunded,
        SourceRedeemedTargetFunded,
    ))]
    BothFunded {
        swap: OngoingSwap<R>,
        target_htlc_location: <R::TargetLedger as Ledger>::HtlcLocation,
        source_htlc_location: <R::SourceLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        swap: OngoingSwap<R>,
        source_htlc_location: <R::SourceLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        swap: OngoingSwap<R>,
        target_htlc_location: <R::TargetLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        swap: OngoingSwap<R>,
        target_htlc_location: <R::TargetLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        swap: OngoingSwap<R>,
        target_redeemed_tx: <R::TargetLedger as swap_protocols::Ledger>::Transaction,
        source_htlc_location: <R::SourceLedger as Ledger>::HtlcLocation,
        secret: Secret,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(rfc003::Error),
}

impl<R: Role> PollSwap<R> for Swap<R> {
    fn poll_start<'s, 'c>(
        state: &'s mut RentToOwn<'s, Start<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterStart<R>>, rfc003::Error> {
        let request = comit_client::rfc003::Request {
            source_asset: state.source_asset.clone(),
            target_asset: state.target_asset.clone(),
            source_ledger: state.source_ledger.clone(),
            target_ledger: state.target_ledger.clone(),
            source_ledger_refund_identity: state.source_ledger_refund_identity.clone().into(),
            target_ledger_success_identity: state.target_ledger_success_identity.clone().into(),
            source_ledger_lock_duration: state.source_ledger_lock_duration.clone(),
            secret_hash: state.secret.clone().into(),
        };

        let response = try_ready!(context.response_event.request_responded(&request).poll());

        let state = state.take();

        match response {
            Ok(swap_accepted) => transition_save!(
                context.state_repo,
                Accepted {
                    swap: OngoingSwap::new(state, swap_accepted),
                }
            ),
            Err(_) => transition_save!(context.state_repo, Final(SwapOutcome::Rejected)),
        }
    }

    fn poll_accepted<'s, 'c>(
        state: &'s mut RentToOwn<'s, Accepted<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAccepted<R>>, rfc003::Error> {
        let source_htlc_location = try_ready!(context
            .ledger_events
            .source_htlc_funded(state.swap.source_htlc_params())
            .poll());

        let state = state.take();

        transition_save!(
            context.state_repo,
            SourceFunded {
                swap: state.swap,
                source_htlc_location,
            }
        )
    }

    fn poll_source_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, SourceFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterSourceFunded<R>>, rfc003::Error> {
        match try_ready!(context
            .ledger_events
            .source_htlc_refunded_target_htlc_funded(
                state.swap.source_htlc_params(),
                state.swap.target_htlc_params(),
                &state.source_htlc_location
            )
            .poll())
        {
            Either::A(_source_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::SourceRefunded))
            }
            Either::B(target_htlc_location) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    BothFunded {
                        swap: state.swap,
                        source_htlc_location: state.source_htlc_location,
                        target_htlc_location,
                    }
                )
            }
        }
    }

    fn poll_both_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, BothFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterBothFunded<R>>, rfc003::Error> {
        if let Async::Ready(redeemed_or_refunded) = context
            .ledger_events
            .target_htlc_redeemed_or_refunded(
                state.swap.target_htlc_params(),
                &state.target_htlc_location,
            )
            .poll()?
        {
            let state = state.take();
            let secret_hash = state.swap.secret.clone().into();
            match redeemed_or_refunded {
                Either::A(target_redeemed_tx) => {
                    match R::TargetLedger::extract_secret(&target_redeemed_tx, &secret_hash) {
                        Some(secret) => transition_save!(
                            context.state_repo,
                            SourceFundedTargetRedeemed {
                                swap: state.swap,
                                target_redeemed_tx,
                                source_htlc_location: state.source_htlc_location,
                                secret,
                            }
                        ),
                        None => {
                            return Err(rfc003::Error::Internal(format!("Somehow reached transition with an invalid secret, transaction: {:?}", target_redeemed_tx).to_string()));
                        }
                    }
                }
                Either::B(_target_refunded_txid) => transition_save!(
                    context.state_repo,
                    SourceFundedTargetRefunded {
                        swap: state.swap,
                        source_htlc_location: state.source_htlc_location,
                    }
                ),
            }
        }

        match try_ready!(context
            .ledger_events
            .source_htlc_redeemed_or_refunded(
                state.swap.source_htlc_params(),
                &state.source_htlc_location
            )
            .poll())
        {
            Either::A(_source_redeemed_tx) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    SourceRedeemedTargetFunded {
                        swap: state.swap,
                        target_htlc_location: state.target_htlc_location,
                    }
                )
            }
            Either::B(_source_refunded_txid) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    SourceRefundedTargetFunded {
                        swap: state.swap,
                        target_htlc_location: state.target_htlc_location,
                    }
                )
            }
        }
    }

    fn poll_source_funded_target_refunded<'s, 'c>(
        state: &'s mut RentToOwn<'s, SourceFundedTargetRefunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterSourceFundedTargetRefunded>, rfc003::Error> {
        match try_ready!(context
            .ledger_events
            .source_htlc_redeemed_or_refunded(
                state.swap.source_htlc_params(),
                &state.source_htlc_location
            )
            .poll())
        {
            Either::A(_source_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRedeemedTargetRefunded)
            ),
            Either::B(_source_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_source_refunded_target_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, SourceRefundedTargetFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterSourceRefundedTargetFunded>, rfc003::Error> {
        match try_ready!(context
            .ledger_events
            .target_htlc_redeemed_or_refunded(
                state.swap.target_htlc_params(),
                &state.target_htlc_location
            )
            .poll())
        {
            Either::A(_target_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRefundedTargetRedeemed)
            ),
            Either::B(_target_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_source_redeemed_target_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, SourceRedeemedTargetFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterSourceRedeemedTargetFunded>, rfc003::Error> {
        match try_ready!(context
            .ledger_events
            .target_htlc_redeemed_or_refunded(
                state.swap.target_htlc_params(),
                &state.target_htlc_location
            )
            .poll())
        {
            Either::A(_target_redeemed_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRedeemed))
            }
            Either::B(_target_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::SourceRedeemedTargetRefunded)
            ),
        }
    }

    fn poll_source_funded_target_redeemed<'s, 'c>(
        state: &'s mut RentToOwn<'s, SourceFundedTargetRedeemed<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterSourceFundedTargetRedeemed>, rfc003::Error> {
        match try_ready!(context
            .ledger_events
            .source_htlc_redeemed_or_refunded(
                state.swap.source_htlc_params(),
                &state.source_htlc_location
            )
            .poll())
        {
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
