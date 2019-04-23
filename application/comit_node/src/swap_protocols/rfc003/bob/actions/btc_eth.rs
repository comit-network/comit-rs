use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::CreateActions,
        bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum,
        state_machine::HtlcParams,
        Action, Actions, LedgerState,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use std::sync::Arc;

impl Actions for bob::State<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity> {
    type ActionKind = bob::ActionKind<
        Accept<Bitcoin, Ethereum>,
        Decline<Bitcoin, Ethereum>,
        (),
        ethereum::ContractDeploy,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
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
        let mut actions = match (alpha_state, beta_state, self.secret) {
            (Funded { htlc_location, .. }, _, Some(secret)) => {
                vec![
                    bob::ActionKind::Redeem(<(Bitcoin, BitcoinQuantity)>::redeem_action(
                        HtlcParams::new_alpha_params(request, response),
                        htlc_location.clone(),
                        self.secret_source.as_ref(),
                        secret,
                    ))
                    .into_action(),
                ]
            }
            (Funded { .. }, NotDeployed, _) => {
                vec![
                    bob::ActionKind::Fund(<(Ethereum, EtherQuantity)>::fund_action(
                        HtlcParams::new_beta_params(request, response),
                    ))
                    .into_action(),
                ]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                bob::ActionKind::Refund(<(Ethereum, EtherQuantity)>::refund_action(
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
