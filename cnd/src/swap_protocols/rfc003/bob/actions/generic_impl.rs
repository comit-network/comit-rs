use crate::swap_protocols::{
    actions::Actions,
    rfc003::{
        actions::{Accept, Action, Decline, FundAction, RedeemAction, RefundAction},
        bob,
        create_swap::HtlcParams,
        LedgerState, SwapCommunication,
    },
};
use std::convert::Infallible;

impl<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> Actions
    for bob::State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>
where
    AL: Clone,
    BL: Clone,
    AA: Clone,
    BA: Clone,
    AH: Clone,
    BH: Clone,
    AI: Clone,
    BI: Clone,
    AT: Clone,
    BT: Clone,
    (BL, BA): FundAction<HtlcParams = HtlcParams<BL, BA, BI>>
        + RefundAction<HtlcParams = HtlcParams<BL, BA, BI>, HtlcLocation = BH, FundTransaction = BT>,
    (AL, AA): RedeemAction<HtlcParams = HtlcParams<AL, AA, AI>, HtlcLocation = AH>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<AL, BL>,
        Decline<AL, BL>,
        Infallible,
        <(BL, BA) as FundAction>::Output,
        <(AL, AA) as RedeemAction>::Output,
        <(BL, BA) as RefundAction>::Output,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let (request, response) = match &self.swap_communication {
            SwapCommunication::Proposed { .. } => {
                return vec![
                    Action::Accept(Accept::new()),
                    Action::Decline(Decline::new()),
                ];
            }
            SwapCommunication::Accepted {
                ref request,
                ref response,
            } => (request, response),
            _ => return vec![],
        };

        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;
        let mut actions = match (alpha_state, beta_state) {
            (Funded { htlc_location, .. }, Redeemed { secret, .. }) => {
                vec![Action::Redeem(<(AL, AA)>::redeem_action(
                    HtlcParams::new_alpha_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source, // Derive identities with this.
                    *secret,              /* Bob uses the secret learned from Alice redeem
                                           * action. */
                ))]
            }
            (Funded { .. }, NotDeployed) => vec![Action::Fund(<(BL, BA)>::fund_action(
                HtlcParams::new_beta_params(request, response),
            ))],
            _ => vec![],
        };

        if let Funded {
            htlc_location,
            fund_transaction,
            ..
        }
        | IncorrectlyFunded {
            htlc_location,
            fund_transaction,
            ..
        } = beta_state
        {
            actions.push(Action::Refund(<(BL, BA)>::refund_action(
                HtlcParams::new_beta_params(request, response),
                htlc_location.clone(),
                &*self.secret_source,
                fund_transaction,
            )))
        }

        actions
    }
}
