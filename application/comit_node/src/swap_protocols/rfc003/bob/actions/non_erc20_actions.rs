use crate::swap_protocols::{
    asset::Asset,
    rfc003::{
        actions::CreateActions,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        state_machine::HtlcParams,
        Action, Actions, Ledger, LedgerState,
    },
};
use std::sync::Arc;

impl<AL, BL, AA, BA> Actions for bob::State<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
    (AL, AA): CreateActions<AL, AA>,
    (BL, BA): CreateActions<BL, BA>,
{
    type ActionKind = bob::ActionKind<
        Accept<AL, BL>,
        Decline<AL, BL>,
        (),
        <(BL, BA) as CreateActions<BL, BA>>::FundActionOutput,
        <(AL, AA) as CreateActions<AL, AA>>::RedeemActionOutput,
        <(BL, BA) as CreateActions<BL, BA>>::RefundActionOutput,
    >;

    fn actions(&self) -> Vec<Action<Self::ActionKind>> {
        let (request, response) = match &self.swap_communication {
            SwapCommunication::Proposed {
                pending_response, ..
            } => {
                return vec![
                    bob::ActionKind::Accept(Accept::new(
                        pending_response.sender.clone(),
                        Arc::clone(&self.secret_source),
                    ))
                    .into_action(),
                    bob::ActionKind::Decline(Decline::new(pending_response.sender.clone()))
                        .into_action(),
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
        let mut actions =
            match (alpha_state, beta_state, self.secret) {
                (Funded { htlc_location, .. }, _, Some(secret)) => {
                    vec![bob::ActionKind::Redeem(<(AL, AA)>::redeem_action(
                        HtlcParams::new_alpha_params(request, response),
                        htlc_location.clone(),
                        &*self.secret_source,
                        secret,
                    ))
                    .into_action()]
                }
                (Funded { .. }, NotDeployed, _) => vec![bob::ActionKind::Fund(
                    <(BL, BA)>::fund_action(HtlcParams::new_beta_params(request, response)),
                )
                .into_action()],
                _ => vec![],
            };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                bob::ActionKind::Refund(<(BL, BA)>::refund_action(
                    HtlcParams::new_beta_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                ))
                .into_action()
                .with_invalid_until(request.beta_expiry),
            )
        }

        actions
    }
}
