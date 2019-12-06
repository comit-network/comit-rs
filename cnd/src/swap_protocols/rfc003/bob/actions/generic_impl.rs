use crate::swap_protocols::{
    actions::Actions,
    asset::Asset,
    rfc003::{
        actions::{Accept, Action, Decline, FundAction, RedeemAction, RefundAction},
        bob,
        state_machine::HtlcParams,
        Ledger, LedgerState, SwapCommunication,
    },
};
use std::convert::Infallible;

impl<AL, BL, AA, BA> Actions for bob::State<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
    (BL, BA): FundAction<BL, BA> + RefundAction<BL, BA>,
    (AL, AA): RedeemAction<AL, AA>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<AL, BL>,
        Decline<AL, BL>,
        Infallible,
        <(BL, BA) as FundAction<BL, BA>>::FundActionOutput,
        <(AL, AA) as RedeemAction<AL, AA>>::RedeemActionOutput,
        <(BL, BA) as RefundAction<BL, BA>>::RefundActionOutput,
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
        let mut actions = match (alpha_state, beta_state, self.secret) {
            (Funded { htlc_location, .. }, _, Some(secret)) => {
                vec![Action::Redeem(<(AL, AA)>::redeem_action(
                    HtlcParams::new_alpha_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                    secret,
                ))]
            }
            (Funded { .. }, NotDeployed, _) => vec![Action::Fund(<(BL, BA)>::fund_action(
                HtlcParams::new_beta_params(request, response),
            ))],
            _ => vec![],
        };

        if let Funded {
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
