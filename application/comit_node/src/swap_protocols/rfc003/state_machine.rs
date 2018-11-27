use comit_client;
use futures::{future::Either, Async};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::sync::Arc;
use swap_protocols::{
    self,
    asset::Asset,
    rfc003::{
        self, events, ledger::Ledger, roles::Role, ExtractSecret, SaveState, Secret, SecretHash,
        SwapOutcome,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, LabelledGeneric)]
pub struct StateMachineResponse<ALSI, BLRI, BLLD> {
    pub alpha_ledger_success_identity: ALSI,
    pub beta_ledger_refund_identity: BLRI,
    pub beta_ledger_lock_duration: BLLD,
}

impl<AL: Ledger, BL: Ledger> From<comit_client::rfc003::AcceptResponseBody<AL, BL>>
    for StateMachineResponse<AL::Identity, BL::Identity, BL::LockDuration>
{
    fn from(accept_response: comit_client::rfc003::AcceptResponseBody<AL, BL>) -> Self {
        Self {
            alpha_ledger_success_identity: accept_response.alpha_ledger_success_identity,
            beta_ledger_refund_identity: accept_response.beta_ledger_refund_identity,
            beta_ledger_lock_duration: accept_response.beta_ledger_lock_duration,
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
    pub alpha_ledger: R::AlphaLedger,
    pub beta_ledger: R::BetaLedger,
    pub alpha_asset: R::AlphaAsset,
    pub beta_asset: R::BetaAsset,
    pub alpha_ledger_success_identity: R::AlphaSuccessHtlcIdentity,
    pub alpha_ledger_refund_identity: R::AlphaRefundHtlcIdentity,
    pub beta_ledger_success_identity: R::BetaSuccessHtlcIdentity,
    pub beta_ledger_refund_identity: R::BetaRefundHtlcIdentity,
    pub alpha_ledger_lock_duration: <R::AlphaLedger as Ledger>::LockDuration,
    pub beta_ledger_lock_duration: <R::BetaLedger as Ledger>::LockDuration,
    pub secret: R::Secret,
}

impl<R: Role> OngoingSwap<R> {
    pub fn new(
        start: Start<R>,
        response: StateMachineResponse<
            R::AlphaSuccessHtlcIdentity,
            R::BetaRefundHtlcIdentity,
            <R::BetaLedger as Ledger>::LockDuration,
        >,
    ) -> Self {
        OngoingSwap {
            alpha_ledger: start.alpha_ledger,
            beta_ledger: start.beta_ledger,
            alpha_asset: start.alpha_asset,
            beta_asset: start.beta_asset,
            alpha_ledger_success_identity: response.alpha_ledger_success_identity,
            alpha_ledger_refund_identity: start.alpha_ledger_refund_identity,
            beta_ledger_success_identity: start.beta_ledger_success_identity,
            beta_ledger_refund_identity: response.beta_ledger_refund_identity,
            alpha_ledger_lock_duration: start.alpha_ledger_lock_duration,
            beta_ledger_lock_duration: response.beta_ledger_lock_duration,
            secret: start.secret,
        }
    }

    pub fn alpha_htlc_params(&self) -> HtlcParams<R::AlphaLedger, R::AlphaAsset> {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: self.alpha_ledger.clone(),
            success_identity: self.alpha_ledger_success_identity.clone().into(),
            refund_identity: self.alpha_ledger_refund_identity.clone().into(),
            lock_duration: self.alpha_ledger_lock_duration.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }

    pub fn beta_htlc_params(&self) -> HtlcParams<R::BetaLedger, R::BetaAsset> {
        HtlcParams {
            asset: self.beta_asset.clone(),
            ledger: self.beta_ledger.clone(),
            success_identity: self.beta_ledger_success_identity.clone().into(),
            refund_identity: self.beta_ledger_refund_identity.clone().into(),
            lock_duration: self.beta_ledger_lock_duration.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct Context<R: Role> {
    pub alpha_ledger_events: Box<events::LedgerEvents<R::AlphaLedger, R::AlphaAsset>>,
    pub beta_ledger_events: Box<events::LedgerEvents<R::BetaLedger, R::BetaAsset>>,
    pub state_repo: Arc<SaveState<R>>,
    pub communication_events: Box<events::CommunicationEvents<R>>,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
#[allow(missing_debug_implementations, clippy::too_many_arguments)]
pub enum Swap<R: Role> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        alpha_ledger_refund_identity: R::AlphaRefundHtlcIdentity,
        beta_ledger_success_identity: R::BetaSuccessHtlcIdentity,
        alpha_ledger: R::AlphaLedger,
        beta_ledger: R::BetaLedger,
        alpha_asset: R::AlphaAsset,
        beta_asset: R::BetaAsset,
        alpha_ledger_lock_duration: <R::AlphaLedger as Ledger>::LockDuration,
        secret: R::Secret,
    },

    #[state_machine_future(transitions(AlphaDeployed))]
    Accepted { swap: OngoingSwap<R> },

    #[state_machine_future(transitions(AlphaFunded, Final))]
    AlphaDeployed {
        swap: OngoingSwap<R>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(AlphaFundedBetaDeployed, Final))]
    AlphaFunded {
        swap: OngoingSwap<R>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    AlphaFundedBetaDeployed {
        swap: OngoingSwap<R>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
        beta_htlc_location: <R::BetaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(
        AlphaFundedBetaRedeemed,
        AlphaFundedBetaRefunded,
        AlphaRefundedBetaFunded,
        AlphaRedeemedBetaFunded,
    ))]
    BothFunded {
        swap: OngoingSwap<R>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
        beta_htlc_location: <R::BetaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    AlphaFundedBetaRefunded {
        swap: OngoingSwap<R>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    AlphaRefundedBetaFunded {
        swap: OngoingSwap<R>,
        beta_htlc_location: <R::BetaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    AlphaRedeemedBetaFunded {
        swap: OngoingSwap<R>,
        beta_htlc_location: <R::BetaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(transitions(Final))]
    AlphaFundedBetaRedeemed {
        swap: OngoingSwap<R>,
        beta_redeemed_tx: <R::BetaLedger as swap_protocols::Ledger>::Transaction,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
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
            alpha_asset: state.alpha_asset.clone(),
            beta_asset: state.beta_asset.clone(),
            alpha_ledger: state.alpha_ledger.clone(),
            beta_ledger: state.beta_ledger.clone(),
            alpha_ledger_refund_identity: state.alpha_ledger_refund_identity.clone().into(),
            beta_ledger_success_identity: state.beta_ledger_success_identity.clone().into(),
            alpha_ledger_lock_duration: state.alpha_ledger_lock_duration.clone(),
            secret_hash: state.secret.clone().into(),
        };

        let response = try_ready!(context
            .communication_events
            .request_responded(&request)
            .poll());

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
        let alpha_htlc_location = try_ready!(context
            .alpha_ledger_events
            .htlc_deployed(state.swap.alpha_htlc_params())
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            AlphaDeployed {
                swap: state.swap,
                alpha_htlc_location,
            }
        )
    }

    fn poll_alpha_deployed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaDeployed<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaDeployed<R>>, rfc003::Error> {
        let _alpha_funding_transaction = try_ready!(context
            .alpha_ledger_events
            .htlc_funded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            AlphaFunded {
                swap: state.swap,
                alpha_htlc_location: state.alpha_htlc_location,
            }
        )
    }

    fn poll_alpha_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaFunded<R>>, rfc003::Error> {
        if let Async::Ready(_alpha_redeemed_or_refunded) = context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll()?
        {
            transition_save!(context.state_repo, Final(SwapOutcome::AlphaRefunded))
        }

        let beta_htlc_location = try_ready!(context
            .beta_ledger_events
            .htlc_deployed(state.swap.beta_htlc_params())
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            AlphaFundedBetaDeployed {
                swap: state.swap,
                alpha_htlc_location: state.alpha_htlc_location,
                beta_htlc_location,
            }
        )
    }

    fn poll_alpha_funded_beta_deployed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaDeployed<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaFundedBetaDeployed<R>>, rfc003::Error> {
        if let Async::Ready(_alpha_redeemed_or_refunded) = context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll()?
        {
            transition_save!(context.state_repo, Final(SwapOutcome::AlphaRefunded))
        }

        let _ = try_ready!(context
            .beta_ledger_events
            .htlc_funded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            BothFunded {
                swap: state.swap,
                alpha_htlc_location: state.alpha_htlc_location,
                beta_htlc_location: state.beta_htlc_location,
            }
        )
    }

    fn poll_both_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, BothFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterBothFunded<R>>, rfc003::Error> {
        if let Async::Ready(redeemed_or_refunded) = context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll()?
        {
            let state = state.take();
            let secret_hash = state.swap.secret.clone().into();
            match redeemed_or_refunded {
                Either::A(beta_redeemed_tx) => {
                    match R::BetaLedger::extract_secret(&beta_redeemed_tx, &secret_hash) {
                        Some(secret) => transition_save!(
                            context.state_repo,
                            AlphaFundedBetaRedeemed {
                                swap: state.swap,
                                beta_redeemed_tx,
                                alpha_htlc_location: state.alpha_htlc_location,
                                secret,
                            }
                        ),
                        None => {
                            return Err(rfc003::Error::Internal(format!("Somehow reached transition with an invalid secret, transaction: {:?}", beta_redeemed_tx).to_string()));
                        }
                    }
                }
                Either::B(_beta_refunded_txid) => transition_save!(
                    context.state_repo,
                    AlphaFundedBetaRefunded {
                        swap: state.swap,
                        alpha_htlc_location: state.alpha_htlc_location,
                    }
                ),
            }
        }

        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll())
        {
            Either::A(_alpha_redeemed_tx) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    AlphaRedeemedBetaFunded {
                        swap: state.swap,
                        beta_htlc_location: state.beta_htlc_location,
                    }
                )
            }
            Either::B(_alpha_refunded_txid) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    AlphaRefundedBetaFunded {
                        swap: state.swap,
                        beta_htlc_location: state.beta_htlc_location,
                    }
                )
            }
        }
    }

    fn poll_alpha_funded_beta_refunded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaRefunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaFundedBetaRefunded>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll())
        {
            Either::A(_alpha_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRedeemedBetaRefunded)
            ),
            Either::B(_alpha_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_alpha_refunded_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRefundedBetaFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaRefundedBetaFunded>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefundedBetaRedeemed)
            ),
            Either::B(_beta_refunded_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRefunded))
            }
        }
    }

    fn poll_alpha_redeemed_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRedeemedBetaFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaRedeemedBetaFunded>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRedeemed))
            }
            Either::B(_beta_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRedeemedBetaRefunded)
            ),
        }
    }

    fn poll_alpha_funded_beta_redeemed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaRedeemed<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaFundedBetaRedeemed>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => {
                transition_save!(context.state_repo, Final(SwapOutcome::BothRedeemed))
            }
            Either::B(_beta_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefundedBetaRedeemed)
            ),
        }
    }
}

impl<R: Role> SwapStates<R> {
    pub fn name(&self) -> String {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => String::from("Start"),
            SS::Accepted { .. } => String::from("Accepted"),
            SS::AlphaDeployed { .. } => String::from("AlphaDeployed"),
            SS::AlphaFunded { .. } => String::from("AlphaFunded"),
            SS::AlphaFundedBetaDeployed { .. } => String::from("AlphaFundedBetaDeployed"),
            SS::BothFunded { .. } => String::from("BothFunded"),
            SS::AlphaFundedBetaRefunded { .. } => String::from("AlphaFundedBetaRefunded"),
            SS::AlphaRefundedBetaFunded { .. } => String::from("AlphaRefundedBetaFunded"),
            SS::AlphaFundedBetaRedeemed { .. } => String::from("AlphaFundedBetaRedeemed"),
            SS::AlphaRedeemedBetaFunded { .. } => String::from("AlphaRedeemedBetaFunded"),
            SS::Error(_) => String::from("Error"),
            SS::Final(_) => String::from("Final"),
        }
    }

    pub fn swap_details(&self) -> Option<SwapDetails<R>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start(ref start) => Some(SwapDetails::<R>::from(start.clone())),
            SS::Accepted(Accepted { ref swap, .. })
            | SS::AlphaDeployed(AlphaDeployed { ref swap, .. })
            | SS::AlphaFunded(AlphaFunded { ref swap, .. })
            | SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { ref swap, .. })
            | SS::BothFunded(BothFunded { ref swap, .. })
            | SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded { ref swap, .. })
            | SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded { ref swap, .. })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed { ref swap, .. })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded { ref swap, .. }) => {
                Some(SwapDetails::<R>::from(swap.clone()))
            }
            SS::Final(Final(ref swap_outcome)) => {
                Some(SwapDetails::<R>::from(swap_outcome.clone()))
            }
            SS::Error(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct SwapDetails<R: Role> {
    pub state_name: String,                   // TODO: Better way
    pub alpha_ledger: Option<R::AlphaLedger>, // TODO: Add Ledgers/Asset to SwapOutcome
    pub beta_ledger: Option<R::BetaLedger>,
    pub alpha_asset: Option<R::AlphaAsset>,
    pub beta_asset: Option<R::BetaAsset>,
    pub alpha_ledger_success_identity: Option<R::AlphaSuccessHtlcIdentity>,
    pub alpha_ledger_refund_identity: Option<R::AlphaRefundHtlcIdentity>,
    pub beta_ledger_success_identity: Option<R::BetaSuccessHtlcIdentity>,
    pub beta_ledger_refund_identity: Option<R::BetaRefundHtlcIdentity>,
    pub alpha_ledger_lock_duration: Option<<R::AlphaLedger as Ledger>::LockDuration>,
    pub beta_ledger_lock_duration: Option<<R::BetaLedger as Ledger>::LockDuration>,
    pub secret: Option<R::Secret>,
    pub revealed_secret: Option<Secret>,
}

impl<R: Role> From<Start<R>> for SwapDetails<R> {
    fn from(start: Start<R>) -> Self {
        SwapDetails {
            state_name: "Start".to_string(),
            alpha_ledger: Some(start.alpha_ledger),
            beta_ledger: Some(start.beta_ledger),
            alpha_asset: Some(start.alpha_asset),
            beta_asset: Some(start.beta_asset),
            alpha_ledger_success_identity: None,
            alpha_ledger_refund_identity: Some(start.alpha_ledger_refund_identity),
            beta_ledger_success_identity: None,
            beta_ledger_refund_identity: None,
            alpha_ledger_lock_duration: Some(start.alpha_ledger_lock_duration),
            beta_ledger_lock_duration: None,
            secret: Some(start.secret),
            revealed_secret: None,
        }
    }
}

impl<R: Role> From<OngoingSwap<R>> for SwapDetails<R> {
    fn from(swap: OngoingSwap<R>) -> Self {
        SwapDetails {
            state_name: "Ongoing".to_string(), // TODO: Actual state name would be nice
            alpha_ledger: Some(swap.alpha_ledger),
            beta_ledger: Some(swap.beta_ledger),
            alpha_asset: Some(swap.alpha_asset),
            beta_asset: Some(swap.beta_asset),
            alpha_ledger_success_identity: Some(swap.alpha_ledger_success_identity),
            alpha_ledger_refund_identity: Some(swap.alpha_ledger_refund_identity),
            beta_ledger_success_identity: Some(swap.beta_ledger_success_identity),
            beta_ledger_refund_identity: Some(swap.beta_ledger_refund_identity),
            alpha_ledger_lock_duration: Some(swap.alpha_ledger_lock_duration),
            beta_ledger_lock_duration: Some(swap.beta_ledger_lock_duration),
            secret: Some(swap.secret),
            revealed_secret: None, // TODO: Extract secret from appropriate states
        }
    }
}

impl<R: Role> From<SwapOutcome> for SwapDetails<R> {
    fn from(swap_outcome: SwapOutcome) -> Self {
        SwapDetails {
            state_name: format!("{:?}", swap_outcome),
            alpha_ledger: None,
            beta_ledger: None,
            alpha_asset: None,
            beta_asset: None,
            alpha_ledger_success_identity: None,
            alpha_ledger_refund_identity: None,
            beta_ledger_success_identity: None,
            beta_ledger_refund_identity: None,
            alpha_ledger_lock_duration: None,
            beta_ledger_lock_duration: None,
            secret: None,
            revealed_secret: None,
        }
    }
}
