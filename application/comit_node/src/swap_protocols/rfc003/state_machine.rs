// TODO: Figure out how to properly place this on the state_machine_future
// derive so that is is forwarded to the generated structs and impl
#![allow(clippy::too_many_arguments)]

use crate::{
    comit_client::SwapReject,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            events::{self, Deployed, Funded, Redeemed, Refunded},
            ledger::Ledger,
            messages::AcceptResponseBody,
            SaveState, SecretHash, Timestamp,
        },
    },
};
use either::Either;
use futures::{future, Async, Future};
use state_machine_future::{RentToOwn, StateMachineFuture};
use std::{fmt, sync::Arc};

#[derive(Clone, Debug)]
pub struct HtlcParams<L: Ledger, A: Asset> {
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: L::Identity,
    pub refund_identity: L::Identity,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<L: Ledger, A: Asset> HtlcParams<L, A> {
    pub fn new_alpha_params<BL: Ledger, BA: Asset>(
        request: &rfc003::messages::Request<L, BL, A, BA>,
        accept_response: &rfc003::messages::AcceptResponseBody<L, BL>,
    ) -> Self {
        HtlcParams {
            asset: request.alpha_asset,
            ledger: request.alpha_ledger,
            redeem_identity: accept_response.alpha_ledger_redeem_identity,
            refund_identity: request.alpha_ledger_refund_identity,
            expiry: request.alpha_expiry,
            secret_hash: request.secret_hash,
        }
    }

    pub fn new_beta_params<AL: Ledger, AA: Asset>(
        request: &rfc003::messages::Request<AL, L, AA, A>,
        accept_response: &rfc003::messages::AcceptResponseBody<AL, L>,
    ) -> Self {
        HtlcParams {
            asset: request.beta_asset,
            ledger: request.beta_ledger,
            redeem_identity: request.beta_ledger_redeem_identity,
            refund_identity: accept_response.beta_ledger_refund_identity,
            expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OngoingSwap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub alpha_ledger_redeem_identity: AL::Identity,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> OngoingSwap<AL, BL, AA, BA> {
    pub fn new(start: Start<AL, BL, AA, BA>, response: AcceptResponseBody<AL, BL>) -> Self {
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
            secret_hash: start.secret_hash,
        }
    }

    pub fn alpha_htlc_params(&self) -> HtlcParams<AL, AA> {
        HtlcParams {
            asset: self.alpha_asset,
            ledger: self.alpha_ledger,
            redeem_identity: self.alpha_ledger_redeem_identity,
            refund_identity: self.alpha_ledger_refund_identity,
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
    }

    pub fn beta_htlc_params(&self) -> HtlcParams<BL, BA> {
        HtlcParams {
            asset: self.beta_asset,
            ledger: self.beta_ledger,
            redeem_identity: self.beta_ledger_redeem_identity,
            refund_identity: self.beta_ledger_refund_identity,
            expiry: self.beta_expiry,
            secret_hash: self.secret_hash,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    Rejected {
        start: Start<AL, BL, AA, BA>,
        rejection_type: SwapReject,
    },
    AlphaRefunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        alpha_refunded: Refunded<AL>,
    },
    AlphaRedeemed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        alpha_redeemed: Redeemed<AL>,
    },
    BothRefunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_or_beta_refunded: Either<Refunded<AL>, Refunded<BL>>,
    },
    BothRedeemed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_or_beta_redeemed: Either<Redeemed<AL>, Redeemed<BL>>,
    },
    AlphaRedeemedBetaRefunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_redeemed_or_beta_refunded: Either<Redeemed<AL>, Refunded<BL>>,
    },
    AlphaRefundedBetaRedeemed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_refunded_or_beta_redeemed: Either<Refunded<AL>, Redeemed<BL>>,
    },
}

#[allow(type_alias_bounds)]
pub type FutureSwapOutcome<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> =
    dyn Future<Item = SwapOutcome<AL, BL, AA, BA>, Error = rfc003::Error> + Send;

#[allow(missing_debug_implementations)]
pub struct Context<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub alpha_ledger_events: Box<dyn events::LedgerEvents<AL, AA>>,
    pub beta_ledger_events: Box<dyn events::LedgerEvents<BL, BA>>,
    pub state_repo: Arc<dyn SaveState<AL, BL, AA, BA>>,
    pub communication_events: Box<dyn events::CommunicationEvents<AL, BL, AA, BA>>,
}

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context", derive(Clone, Debug, PartialEq))]
#[allow(missing_debug_implementations)]
pub enum Swap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    #[state_machine_future(start, transitions(Accepted, Final))]
    Start {
        alpha_ledger_refund_identity: AL::Identity,
        beta_ledger_redeem_identity: BL::Identity,
        alpha_ledger: AL,
        beta_ledger: BL,
        alpha_asset: AA,
        beta_asset: BA,
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
        secret_hash: SecretHash,
    },

    #[state_machine_future(transitions(AlphaDeployed))]
    Accepted { swap: OngoingSwap<AL, BL, AA, BA> },

    #[state_machine_future(transitions(AlphaFunded, Final))]
    AlphaDeployed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
    },

    #[state_machine_future(transitions(AlphaFundedBetaDeployed, Final))]
    AlphaFunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
    },

    #[state_machine_future(transitions(BothFunded, Final))]
    AlphaFundedBetaDeployed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
    },

    #[state_machine_future(transitions(
        AlphaFundedBetaRedeemed,
        AlphaFundedBetaRefunded,
        AlphaRefundedBetaFunded,
        AlphaRedeemedBetaFunded,
    ))]
    BothFunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
    },

    #[state_machine_future(transitions(Final))]
    AlphaFundedBetaRefunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        beta_refund_transaction: Refunded<BL>,
    },

    #[state_machine_future(transitions(Final))]
    AlphaRefundedBetaFunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_refunded: Refunded<AL>,
    },

    #[state_machine_future(transitions(Final))]
    AlphaRedeemedBetaFunded {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        alpha_redeemed: Redeemed<AL>,
    },

    #[state_machine_future(transitions(Final))]
    AlphaFundedBetaRedeemed {
        swap: OngoingSwap<AL, BL, AA, BA>,
        alpha_deployed: Deployed<AL>,
        alpha_funded: Funded<AL, AA>,
        beta_deployed: Deployed<BL>,
        beta_funded: Funded<BL, BA>,
        beta_redeem_transaction: Redeemed<BL>,
    },

    #[state_machine_future(ready)]
    Final(SwapOutcome<AL, BL, AA, BA>),

    #[state_machine_future(error)]
    Error(rfc003::Error),
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> PollSwap<AL, BL, AA, BA>
    for Swap<AL, BL, AA, BA>
{
    fn poll_start<'s, 'c>(
        state: &'s mut RentToOwn<'s, Start<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterStart<AL, BL, AA, BA>>, rfc003::Error> {
        let request = rfc003::messages::Request {
            alpha_asset: state.alpha_asset,
            beta_asset: state.beta_asset,
            alpha_ledger: state.alpha_ledger,
            beta_ledger: state.beta_ledger,
            alpha_ledger_refund_identity: state.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: state.beta_ledger_redeem_identity,
            alpha_expiry: state.alpha_expiry,
            beta_expiry: state.beta_expiry,
            secret_hash: state.secret_hash,
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
        state: &'s mut RentToOwn<'s, Accepted<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAccepted<AL, BL, AA, BA>>, rfc003::Error> {
        let alpha_deployed = try_ready!(context
            .alpha_ledger_events
            .htlc_deployed(state.swap.alpha_htlc_params())
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            AlphaDeployed {
                swap: state.swap,
                alpha_deployed,
            }
        )
    }

    fn poll_alpha_deployed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaDeployed<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaDeployed<AL, BL, AA, BA>>, rfc003::Error> {
        let alpha_funded = try_ready!(context
            .alpha_ledger_events
            .htlc_funded(state.swap.alpha_htlc_params(), &state.alpha_deployed)
            .poll());
        let state = state.take();

        if alpha_funded
            .asset
            .equal_or_greater_value(&state.swap.alpha_asset)
        {
            transition_save!(
                context.state_repo,
                AlphaFunded {
                    swap: state.swap,
                    alpha_funded,
                    alpha_deployed: state.alpha_deployed,
                }
            )
        } else {
            Err(rfc003::Error::InsufficientFunding)
        }
    }

    fn poll_alpha_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFunded<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaFunded<AL, BL, AA, BA>>, rfc003::Error> {
        if let Async::Ready(alpha_redeemed_or_refunded) = context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.alpha_htlc_params(),
                &state.alpha_deployed,
                &state.alpha_funded,
            )
            .poll()?
        {
            let state = state.take();
            match alpha_redeemed_or_refunded {
                future::Either::A(redeem_transaction) => transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        alpha_redeemed: redeem_transaction
                    })
                ),
                future::Either::B(refund_transaction) => transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        alpha_refunded: refund_transaction
                    })
                ),
            }
        }

        let beta_deployed = try_ready!(context
            .beta_ledger_events
            .htlc_deployed(state.swap.beta_htlc_params())
            .poll());
        let state = state.take();
        transition_save!(
            context.state_repo,
            AlphaFundedBetaDeployed {
                swap: state.swap,
                alpha_funded: state.alpha_funded,
                alpha_deployed: state.alpha_deployed,
                beta_deployed
            }
        )
    }

    fn poll_alpha_funded_beta_deployed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaDeployed<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaFundedBetaDeployed<AL, BL, AA, BA>>, rfc003::Error> {
        if let Async::Ready(alpha_redeemed_or_refunded) = context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.alpha_htlc_params(),
                &state.alpha_deployed,
                &state.alpha_funded,
            )
            .poll()?
        {
            let state = state.take();
            match alpha_redeemed_or_refunded {
                future::Either::A(redeem_transaction) => transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        alpha_redeemed: redeem_transaction
                    })
                ),
                future::Either::B(refund_transaction) => transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        alpha_refunded: refund_transaction
                    })
                ),
            }
        }

        let beta_funded = try_ready!(context
            .beta_ledger_events
            .htlc_funded(state.swap.beta_htlc_params(), &state.beta_deployed)
            .poll());
        let state = state.take();

        if beta_funded
            .asset
            .equal_or_greater_value(&state.swap.beta_asset)
        {
            transition_save!(
                context.state_repo,
                BothFunded {
                    swap: state.swap,
                    alpha_funded: state.alpha_funded,
                    alpha_deployed: state.alpha_deployed,
                    beta_deployed: state.beta_deployed,
                    beta_funded
                }
            )
        } else {
            Err(rfc003::Error::InsufficientFunding)
        }
    }

    fn poll_both_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, BothFunded<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterBothFunded<AL, BL, AA, BA>>, rfc003::Error> {
        if let Async::Ready(redeemed_or_refunded) = context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.beta_htlc_params(),
                &state.beta_deployed,
                &state.beta_funded,
            )
            .poll()?
        {
            let state = state.take();
            match redeemed_or_refunded {
                future::Either::A(beta_redeem_transaction) => transition_save!(
                    context.state_repo,
                    AlphaFundedBetaRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        beta_redeem_transaction,
                    }
                ),
                future::Either::B(beta_refund_transaction) => transition_save!(
                    context.state_repo,
                    AlphaFundedBetaRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        beta_refund_transaction,
                    }
                ),
            }
        }

        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.alpha_htlc_params(),
                &state.alpha_deployed,
                &state.alpha_funded
            )
            .poll())
        {
            future::Either::A(alpha_redeemed) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    AlphaRedeemedBetaFunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_redeemed,
                    }
                )
            }
            future::Either::B(alpha_refunded) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    AlphaRefundedBetaFunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_refunded,
                    }
                )
            }
        }
    }

    fn poll_alpha_funded_beta_refunded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaRefunded<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaFundedBetaRefunded<AL, BL, AA, BA>>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.alpha_htlc_params(),
                &state.alpha_deployed,
                &state.alpha_funded
            )
            .poll())
        {
            future::Either::A(alpha_redeemed) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_redeemed_or_beta_refunded: Either::Left(alpha_redeemed),
                    })
                )
            }
            future::Either::B(alpha_refunded) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::BothRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_or_beta_refunded: Either::Left(alpha_refunded),
                    })
                )
            }
        }
    }

    fn poll_alpha_refunded_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRefundedBetaFunded<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaRefundedBetaFunded<AL, BL, AA, BA>>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.beta_htlc_params(),
                &state.beta_deployed,
                &state.beta_funded
            )
            .poll())
        {
            future::Either::A(beta_redeemed) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_refunded_or_beta_redeemed: Either::Right(beta_redeemed),
                    })
                )
            }
            future::Either::B(beta_refunded) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::BothRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_or_beta_refunded: Either::Right(beta_refunded),
                    })
                )
            }
        }
    }

    fn poll_alpha_redeemed_beta_funded<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaRedeemedBetaFunded<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaRedeemedBetaFunded<AL, BL, AA, BA>>, rfc003::Error> {
        match try_ready!(context
            .beta_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.beta_htlc_params(),
                &state.beta_deployed,
                &state.beta_funded
            )
            .poll())
        {
            future::Either::A(beta_redeemed) => {
                let state = state.take();
                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::BothRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_or_beta_redeemed: Either::Right(beta_redeemed),
                    })
                )
            }
            future::Either::B(beta_refunded) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRedeemedBetaRefunded {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_redeemed_or_beta_refunded: Either::Right(beta_refunded),
                    })
                )
            }
        }
    }

    fn poll_alpha_funded_beta_redeemed<'s, 'c>(
        state: &'s mut RentToOwn<'s, AlphaFundedBetaRedeemed<AL, BL, AA, BA>>,
        context: &'c mut RentToOwn<'c, Context<AL, BL, AA, BA>>,
    ) -> Result<Async<AfterAlphaFundedBetaRedeemed<AL, BL, AA, BA>>, rfc003::Error> {
        match try_ready!(context
            .alpha_ledger_events
            .htlc_redeemed_or_refunded(
                state.swap.alpha_htlc_params(),
                &state.alpha_deployed,
                &state.alpha_funded
            )
            .poll())
        {
            future::Either::A(alpha_redeemed) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::BothRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_or_beta_redeemed: Either::Left(alpha_redeemed),
                    })
                )
            }
            future::Either::B(alpha_refunded) => {
                let state = state.take();

                transition_save!(
                    context.state_repo,
                    Final(SwapOutcome::AlphaRefundedBetaRedeemed {
                        swap: state.swap,
                        alpha_deployed: state.alpha_deployed,
                        alpha_funded: state.alpha_funded,
                        beta_deployed: state.beta_deployed,
                        beta_funded: state.beta_funded,
                        alpha_refunded_or_beta_redeemed: Either::Left(alpha_refunded),
                    })
                )
            }
        }
    }
}

macro_rules! impl_display {
    ($state:ident) => {
        impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> fmt::Display for $state<AL, BL, AA, BA> {
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
