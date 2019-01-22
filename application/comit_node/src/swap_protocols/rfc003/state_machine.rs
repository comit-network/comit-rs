#![allow(clippy::too_many_arguments)] // TODO: Figure out how to properly place this on the state_machine_future derive so that is is forwarded to the generated structs and impl

use crate::{
    comit_client::{self, SwapReject},
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self, events, ledger::Ledger, RedeemTransaction, Role, SaveState, SecretHash, Timestamp,
        },
    },
};
use futures::{future::Either, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::{fmt, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, LabelledGeneric)]
pub struct StateMachineResponse<ALSI, BLRI> {
    pub alpha_ledger_redeem_identity: ALSI,
    pub beta_ledger_refund_identity: BLRI,
}

impl<AL: Ledger, BL: Ledger> From<comit_client::rfc003::AcceptResponseBody<AL, BL>>
    for StateMachineResponse<AL::Identity, BL::Identity>
{
    fn from(accept_response: comit_client::rfc003::AcceptResponseBody<AL, BL>) -> Self {
        Self {
            alpha_ledger_redeem_identity: accept_response.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity: accept_response.beta_ledger_refund_identity,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HtlcParams<L: Ledger, A: Asset> {
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: L::Identity,
    pub refund_identity: L::Identity,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OngoingSwap<R: Role> {
    pub alpha_ledger: R::AlphaLedger,
    pub beta_ledger: R::BetaLedger,
    pub alpha_asset: R::AlphaAsset,
    pub beta_asset: R::BetaAsset,
    pub alpha_ledger_redeem_identity: R::AlphaRedeemHtlcIdentity,
    pub alpha_ledger_refund_identity: R::AlphaRefundHtlcIdentity,
    pub beta_ledger_redeem_identity: R::BetaRedeemHtlcIdentity,
    pub beta_ledger_refund_identity: R::BetaRefundHtlcIdentity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret: R::Secret,
    pub role: R,
}

impl<R: Role> OngoingSwap<R> {
    pub fn new(
        start: Start<R>,
        response: StateMachineResponse<R::AlphaRedeemHtlcIdentity, R::BetaRefundHtlcIdentity>,
    ) -> Self {
        OngoingSwap {
            alpha_ledger: start.alpha_ledger,
            beta_ledger: start.beta_ledger,
            alpha_asset: start.alpha_asset,
            beta_asset: start.beta_asset,
            alpha_ledger_redeem_identity: response.alpha_ledger_redeem_identity,
            alpha_ledger_refund_identity: start.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: start.beta_ledger_redeem_identity,
            beta_ledger_refund_identity: response.beta_ledger_refund_identity,
            alpha_expiry: start.alpha_expiry,
            beta_expiry: start.beta_expiry,
            secret: start.secret,
            role: start.role,
        }
    }

    pub fn alpha_htlc_params(&self) -> HtlcParams<R::AlphaLedger, R::AlphaAsset> {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: self.alpha_ledger.clone(),
            redeem_identity: self.alpha_ledger_redeem_identity.clone().into(),
            refund_identity: self.alpha_ledger_refund_identity.clone().into(),
            expiry: self.alpha_expiry.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }

    pub fn beta_htlc_params(&self) -> HtlcParams<R::BetaLedger, R::BetaAsset> {
        HtlcParams {
            asset: self.beta_asset.clone(),
            ledger: self.beta_ledger.clone(),
            redeem_identity: self.beta_ledger_redeem_identity.clone().into(),
            refund_identity: self.beta_ledger_refund_identity.clone().into(),
            expiry: self.beta_expiry.clone(),
            secret_hash: self.secret.clone().into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome<R: Role> {
    Rejected {
        start: Start<R>,
        rejection_type: SwapReject,
    },
    AlphaRefunded {
        swap: OngoingSwap<R>,
    },
    BothRefunded {
        swap: OngoingSwap<R>,
    },
    BothRedeemed {
        swap: OngoingSwap<R>,
    },
    AlphaRedeemedBetaRefunded {
        swap: OngoingSwap<R>,
    },
    AlphaRefundedBetaRedeemed {
        swap: OngoingSwap<R>,
    },
}

#[allow(type_alias_bounds)]
pub type FutureSwapOutcome<R: Role> =
    dyn Future<Item = SwapOutcome<R>, Error = rfc003::Error> + Send;

#[allow(missing_debug_implementations)]
pub struct Context<R: Role> {
    pub alpha_ledger_events: Box<dyn events::LedgerEvents<R::AlphaLedger, R::AlphaAsset>>,
    pub beta_ledger_events: Box<dyn events::LedgerEvents<R::BetaLedger, R::BetaAsset>>,
    pub state_repo: Arc<dyn SaveState<R>>,
    pub communication_events: Box<dyn events::CommunicationEvents<R>>,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
#[allow(missing_debug_implementations)]
pub enum Swap<R: Role> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        alpha_ledger_refund_identity: R::AlphaRefundHtlcIdentity,
        beta_ledger_redeem_identity: R::BetaRedeemHtlcIdentity,
        alpha_ledger: R::AlphaLedger,
        beta_ledger: R::BetaLedger,
        alpha_asset: R::AlphaAsset,
        beta_asset: R::BetaAsset,
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
        secret: R::Secret,
        role: R,
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
        beta_redeemed_tx: RedeemTransaction<R::BetaLedger>,
        alpha_htlc_location: <R::AlphaLedger as Ledger>::HtlcLocation,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome<R>),

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
            beta_ledger_redeem_identity: state.beta_ledger_redeem_identity.clone().into(),
            alpha_expiry: state.alpha_expiry.clone(),
            beta_expiry: state.beta_expiry.clone(),
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
            Err(rejection_type) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::Rejected {
                    start: state,
                    rejection_type
                })
            ),
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
            transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefunded {
                    swap: state.take().swap
                })
            )
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
            transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefunded {
                    swap: state.take().swap
                })
            )
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
            match redeemed_or_refunded {
                Either::A(beta_redeemed_tx) => transition_save!(
                    context.state_repo,
                    AlphaFundedBetaRedeemed {
                        swap: state.swap,
                        beta_redeemed_tx,
                        alpha_htlc_location: state.alpha_htlc_location,
                    }
                ),
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
    ) -> Result<Async<AfterAlphaFundedBetaRefunded<R>>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll())
        {
            Either::A(_alpha_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                    swap: state.take().swap
                })
            ),
            Either::B(_alpha_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::BothRefunded {
                    swap: state.take().swap
                })
            ),
        }
    }

    fn poll_alpha_refunded_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRefundedBetaFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaRefundedBetaFunded<R>>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                    swap: state.take().swap
                })
            ),
            Either::B(_beta_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::BothRefunded {
                    swap: state.take().swap
                })
            ),
        }
    }

    fn poll_alpha_redeemed_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRedeemedBetaFunded<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaRedeemedBetaFunded<R>>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(state.swap.beta_htlc_params(), &state.beta_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::BothRedeemed {
                    swap: state.take().swap
                })
            ),
            Either::B(_beta_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                    swap: state.take().swap
                })
            ),
        }
    }

    fn poll_alpha_funded_beta_redeemed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaRedeemed<R>>,
        context: &'c mut RentToOwn<'c, Context<R>>,
    ) -> Result<Async<AfterAlphaFundedBetaRedeemed<R>>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(state.swap.alpha_htlc_params(), &state.alpha_htlc_location)
            .poll())
        {
            Either::A(_beta_redeemed_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::BothRedeemed {
                    swap: state.take().swap
                })
            ),
            Either::B(_beta_refunded_txid) => transition_save!(
                context.state_repo,
                Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                    swap: state.take().swap
                })
            ),
        }
    }
}

macro_rules! impl_display {
    ($state:ident) => {
        impl<R: Role> fmt::Display for $state<R> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                write!(f, stringify!($state))
            }
        }
    };
}

impl_display!(Start);
impl_display!(Accepted);
impl_display!(AlphaDeployed);
impl_display!(AlphaFunded);
impl_display!(AlphaFundedBetaDeployed);
impl_display!(BothFunded);
impl_display!(AlphaFundedBetaRefunded);
impl_display!(AlphaRefundedBetaFunded);
impl_display!(AlphaFundedBetaRedeemed);
impl_display!(AlphaRedeemedBetaFunded);
impl_display!(Final);

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
            SS::Final(Final(SwapOutcome::Rejected { .. })) => String::from("Rejected"),
            SS::Final(Final(SwapOutcome::AlphaRefunded { .. })) => String::from("AlphaRefunded"),
            SS::Final(Final(SwapOutcome::BothRefunded { .. })) => String::from("BothRefunded"),
            SS::Final(Final(SwapOutcome::BothRedeemed { .. })) => String::from("BothRedeemed"),
            SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded { .. })) => {
                String::from("AlphaRedeemedBetaRefunded")
            }
            SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed { .. })) => {
                String::from("AlphaRefundedBetaRedeemed")
            }
            SS::Error(_) => String::from("Error"),
        }
    }

    pub fn start_state(&self) -> Result<Start<R>, Error> {
        use self::SwapStates as SS;
        match *self {
            SS::Start(ref start) | SS::Final(Final(SwapOutcome::Rejected { ref start, .. })) => {
                Ok(start.clone())
            }
            SS::Accepted(Accepted { ref swap, .. })
            | SS::AlphaDeployed(AlphaDeployed { ref swap, .. })
            | SS::AlphaFunded(AlphaFunded { ref swap, .. })
            | SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { ref swap, .. })
            | SS::BothFunded(BothFunded { ref swap, .. })
            | SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded { ref swap, .. })
            | SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded { ref swap, .. })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed { ref swap, .. })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded { ref swap, .. })
            | SS::Final(Final(SwapOutcome::AlphaRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::BothRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::BothRedeemed { ref swap }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed { ref swap })) => Ok(Start {
                alpha_ledger: swap.alpha_ledger.clone(),
                beta_ledger: swap.beta_ledger.clone(),
                alpha_asset: swap.alpha_asset.clone(),
                beta_asset: swap.beta_asset.clone(),
                alpha_ledger_refund_identity: swap.alpha_ledger_refund_identity.clone(),
                beta_ledger_redeem_identity: swap.beta_ledger_redeem_identity.clone(),
                alpha_expiry: swap.alpha_expiry.clone(),
                beta_expiry: swap.beta_expiry.clone(),
                secret: swap.secret.clone(),
                role: swap.role.clone(),
            }),
            SS::Error(ref e) => Err(e.clone()),
        }
    }

    pub fn beta_expiry(&self) -> Option<Timestamp> {
        use self::SwapStates as SS;
        match *self {
            SS::Accepted(Accepted { ref swap, .. })
            | SS::AlphaDeployed(AlphaDeployed { ref swap, .. })
            | SS::AlphaFunded(AlphaFunded { ref swap, .. })
            | SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { ref swap, .. })
            | SS::BothFunded(BothFunded { ref swap, .. })
            | SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded { ref swap, .. })
            | SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded { ref swap, .. })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed { ref swap, .. })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded { ref swap, .. })
            | SS::Final(Final(SwapOutcome::AlphaRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::BothRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::BothRedeemed { ref swap }))
            | SS::Final(Final(SwapOutcome::AlphaRedeemedBetaRefunded { ref swap }))
            | SS::Final(Final(SwapOutcome::AlphaRefundedBetaRedeemed { ref swap })) => {
                Some(swap.beta_expiry.clone())
            }
            SS::Start(ref start) => Some(start.beta_expiry),
            SS::Final(_) | SS::Error(_) => None,
        }
    }
}
