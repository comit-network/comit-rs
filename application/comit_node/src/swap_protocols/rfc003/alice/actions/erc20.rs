use crate::swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        actions::{erc20, non_erc20::CreateActions, Actions},
        alice::{self, SwapCommunication},
        ethereum,
        state_machine::HtlcParams,
        Action, Ledger, LedgerState,
    },
};
use ethereum_support::Erc20Token;

impl<BL, BA> Actions for alice::State<Ethereum, BL, Erc20Token, BA>
where
    BL: Ledger,
    BA: Asset,
    (BL, BA): CreateActions<BL, BA>,
{
    type ActionKind = alice::ActionKind<
        ethereum::ContractDeploy,
        ethereum::SendTransaction,
        <(BL, BA) as CreateActions<BL, BA>>::RedeemActionOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Action<Self::ActionKind>> {
        let (request, response) = match self.swap_communication {
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
            match alpha_state {
                NotDeployed => vec![alice::ActionKind::Deploy(erc20::deploy_action(
                    HtlcParams::new_alpha_params(request, response),
                ))
                .into_action()],
                Deployed { htlc_location, .. } => {
                    vec![alice::ActionKind::Fund(erc20::fund_action(
                        HtlcParams::new_alpha_params(request, response),
                        request.alpha_asset.token_contract,
                        *htlc_location,
                    ))
                    .into_action()]
                }
                Funded { htlc_location, .. } => vec![alice::ActionKind::Refund(
                    erc20::refund_action(request.alpha_ledger.network, *htlc_location),
                )
                .into_action()
                .with_invalid_until(request.alpha_expiry)],
                _ => vec![],
            };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                alice::ActionKind::Redeem(<(BL, BA)>::redeem_action(
                    HtlcParams::new_beta_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                    self.secret_source.secret(),
                ))
                .into_action(),
            );
        }
        actions
    }
}

impl<AL, AA> Actions for alice::State<AL, Ethereum, AA, Erc20Token>
where
    AL: Ledger,
    AA: Asset,
    (AL, AA): CreateActions<AL, AA>,
{
    type ActionKind = alice::ActionKind<
        (),
        <(AL, AA) as CreateActions<AL, AA>>::FundActionOutput,
        ethereum::SendTransaction,
        <(AL, AA) as CreateActions<AL, AA>>::RefundActionOutput,
    >;

    fn actions(&self) -> Vec<Action<Self::ActionKind>> {
        let (request, response) = match self.swap_communication {
            SwapCommunication::Accepted {
                ref request,
                ref response,
            } => (request, response),
            _ => return vec![],
        };
        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;

        let mut actions = match alpha_state {
            NotDeployed => vec![alice::ActionKind::Fund(<(AL, AA)>::fund_action(
                HtlcParams::new_alpha_params(request, response),
            ))
            .into_action()],
            Funded { htlc_location, .. } => {
                vec![alice::ActionKind::Refund(<(AL, AA)>::refund_action(
                    HtlcParams::new_alpha_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                ))
                .into_action()
                .with_invalid_until(request.alpha_expiry)]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                alice::ActionKind::Redeem(erc20::redeem_action(
                    *htlc_location,
                    self.secret_source.secret(),
                    request.beta_ledger.network,
                ))
                .into_action(),
            );
        }
        actions
    }
}
