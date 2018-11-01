use futures::{future::Either, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::sync::Arc;
use swap_protocols::rfc003::{
    self, events, ledger::Ledger, messages::Request, AcceptResponse, SaveState, Secret, SecretHash,
    SwapOutcome,
};

pub trait Futures<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone>:
    Send
{
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<events::Response<SL, TL>>;

    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA, S>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<events::Funded<SL>>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA, S>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcLocation,
    ) -> &mut Box<events::SourceRefundedOrTargetFunded<SL, TL>>;

    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_id: &TL::HtlcLocation,
    ) -> &mut Box<events::RedeemedOrRefunded<TL>>;

    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_id: &SL::HtlcLocation,
    ) -> &mut Box<events::RedeemedOrRefunded<SL>>;
}

#[allow(missing_debug_implementations)]
pub struct Context<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone> {
    pub futures: Box<Futures<SL, TL, SA, TA, S>>,
    pub state_repo: Arc<SaveState<SL, TL, SA, TA, S>>,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
#[allow(missing_debug_implementations)]
pub enum Swap<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        source_identity: SL::HtlcIdentity,
        target_identity: TL::HtlcIdentity,
        source_ledger: SL,
        target_ledger: TL,
        source_asset: SA,
        target_asset: TA,
        source_ledger_lock_duration: SL::LockDuration,
        secret: S,
    },

    #[state_machine_future(transitions(SourceFunded))]
    Accepted {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    SourceFunded {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcLocation,
    },

    #[state_machine_future(transitions(
        SourceFundedTargetRefunded,
        SourceRefundedTargetFunded,
        SourceRedeemedTargetFunded,
        SourceFundedTargetRedeemed
    ))]
    BothFunded {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcLocation,
        source_htlc_id: SL::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRefunded {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        source_htlc_id: SL::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceRefundedTargetFunded {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    SourceRedeemedTargetFunded {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        target_htlc_id: TL::HtlcLocation,
        source_htlc_id: SL::HtlcLocation,
        secret: Secret,
    },

    #[state_machine_future(transitions(Final))]
    SourceFundedTargetRedeemed {
        start: Start<SL, TL, SA, TA, S>,
        response: AcceptResponse<SL, TL>,
        target_redeemed_txid: TL::TxId,
        source_htlc_id: SL::HtlcLocation,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome),

    #[state_machine_future(error)]
    Error(rfc003::Error),
}

impl<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone>
    PollSwap<SL, TL, SA, TA, S> for Swap<SL, TL, SA, TA, S>
{
    fn poll_start<'smf_poll>(
        state: &'smf_poll mut RentToOwn<'smf_poll, Start<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterStart<SL, TL, SA, TA, S>>, rfc003::Error> {
        let request = Request {
            source_asset: state.source_asset.clone(),
            target_asset: state.target_asset.clone(),
            source_ledger: state.source_ledger.clone(),
            target_ledger: state.target_ledger.clone(),
            source_ledger_refund_identity: state.source_identity.clone().into(),
            target_ledger_success_identity: state.target_identity.clone().into(),
            source_ledger_lock_duration: state.source_ledger_lock_duration.clone(),
            secret_hash: state.secret.clone().into(),
        };

        let response = try_ready!(context.futures.request_responded(&request).poll());

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
        state: &'smf_poll mut RentToOwn<'smf_poll, Accepted<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterAccepted<SL, TL, SA, TA, S>>, rfc003::Error> {
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
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFunded<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterSourceFunded<SL, TL, SA, TA, S>>, rfc003::Error> {
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
        state: &'smf_poll mut RentToOwn<'smf_poll, BothFunded<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterBothFunded<SL, TL, SA, TA, S>>, rfc003::Error> {
        if let Async::Ready(redeemed_or_refunded) = context
            .futures
            .source_htlc_redeemed_or_refunded(&state.source_htlc_id)
            .poll()?
        {
            let state = state.take();
            match redeemed_or_refunded {
                Either::A(_source_redeemed_txid) => {
                    let bytes = b"hello world, you are beautiful!!"; //TODO get the secret from somewhere
                    let secret = Secret::from(*bytes);
                    transition_save!(
                        context.state_repo,
                        SourceRedeemedTargetFunded {
                            start: state.start,
                            response: state.response,
                            target_htlc_id: state.target_htlc_id,
                            source_htlc_id: state.source_htlc_id,
                            secret,
                        }
                    )
                }
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
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRefunded<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterSourceFundedTargetRefunded>, rfc003::Error> {
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
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRefundedTargetFunded<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterSourceRefundedTargetFunded>, rfc003::Error> {
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
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceRedeemedTargetFunded<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterSourceRedeemedTargetFunded>, rfc003::Error> {
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
        state: &'smf_poll mut RentToOwn<'smf_poll, SourceFundedTargetRedeemed<SL, TL, SA, TA, S>>,
        context: &mut Context<SL, TL, SA, TA, S>,
    ) -> Result<Async<AfterSourceFundedTargetRedeemed>, rfc003::Error> {
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
