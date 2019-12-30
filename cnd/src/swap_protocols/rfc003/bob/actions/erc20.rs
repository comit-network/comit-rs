use crate::{
    ethereum::Erc20Token,
    swap_protocols::{
        actions::{ethereum, Actions},
        asset::Asset,
        ledger::Ethereum,
        rfc003::{
            actions::{erc20, Accept, Action, Decline, FundAction, RedeemAction, RefundAction},
            bob,
            state_machine::HtlcParams,
            Ledger, LedgerState, SwapCommunication,
        },
    },
};
use std::convert::Infallible;

impl<AL, AA> Actions for bob::State<AL, Ethereum, AA, Erc20Token>
where
    AL: Ledger,
    AA: Asset,
    (AL, AA): RedeemAction<AL, AA>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<AL, Ethereum>,
        Decline<AL, Ethereum>,
        ethereum::DeployContract,
        ethereum::CallContract,
        <(AL, AA) as RedeemAction<AL, AA>>::RedeemActionOutput,
        ethereum::CallContract,
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
            (Funded { .. }, NotDeployed, _) => vec![Action::Deploy(erc20::deploy_action(
                HtlcParams::new_beta_params(request, response),
            ))],
            (Funded { .. }, Deployed { htlc_location, .. }, _) => {
                vec![Action::Fund(erc20::fund_action(
                    HtlcParams::new_beta_params(request, response),
                    request.beta_asset.token_contract,
                    *htlc_location,
                ))]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(Action::Refund(erc20::refund_action(
                request.beta_ledger.chain_id,
                request.beta_expiry,
                *htlc_location,
            )));
        }
        actions
    }
}

impl<BL, BA> Actions for bob::State<Ethereum, BL, Erc20Token, BA>
where
    BL: Ledger,
    BA: Asset,
    (BL, BA): FundAction<BL, BA> + RefundAction<BL, BA>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<Ethereum, BL>,
        Decline<Ethereum, BL>,
        Infallible,
        <(BL, BA) as FundAction<BL, BA>>::FundActionOutput,
        ethereum::CallContract,
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
            (Funded { htlc_location, .. }, _, Some(secret)) => vec![Action::Redeem(
                erc20::redeem_action(*htlc_location, secret, request.alpha_ledger.chain_id),
            )],
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
