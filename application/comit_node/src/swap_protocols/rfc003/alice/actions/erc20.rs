use crate::swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        actions::{erc20, Actions, FundAction, RedeemAction, RefundAction},
        alice::{self, SwapCommunication},
        ethereum,
        state_machine::HtlcParams,
        Ledger, LedgerState,
    },
};
use ethereum_support::Erc20Token;

impl<BL, BA> Actions for alice::State<Ethereum, BL, Erc20Token, BA>
where
    BL: Ledger,
    BA: Asset,
    (BL, BA): RedeemAction<BL, BA>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = alice::ActionKind<
        ethereum::ContractDeploy,
        ethereum::CallContract,
        <(BL, BA) as RedeemAction<BL, BA>>::RedeemActionOutput,
        ethereum::CallContract,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
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
            NotDeployed => vec![alice::ActionKind::Deploy(erc20::deploy_action(
                HtlcParams::new_alpha_params(request, response),
            ))],
            Deployed { htlc_location, .. } => vec![alice::ActionKind::Fund(erc20::fund_action(
                HtlcParams::new_alpha_params(request, response),
                request.alpha_asset.token_contract,
                *htlc_location,
            ))],
            Funded { htlc_location, .. } => vec![alice::ActionKind::Refund(erc20::refund_action(
                request.alpha_ledger.network,
                request.alpha_expiry,
                *htlc_location,
            ))],
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(alice::ActionKind::Redeem(<(BL, BA)>::redeem_action(
                HtlcParams::new_beta_params(request, response),
                htlc_location.clone(),
                &*self.secret_source,
                self.secret_source.secret(),
            )));
        }
        actions
    }
}

impl<AL, AA> Actions for alice::State<AL, Ethereum, AA, Erc20Token>
where
    AL: Ledger,
    AA: Asset,
    (AL, AA): FundAction<AL, AA>,
    (AL, AA): RefundAction<AL, AA>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = alice::ActionKind<
        (),
        <(AL, AA) as FundAction<AL, AA>>::FundActionOutput,
        ethereum::CallContract,
        <(AL, AA) as RefundAction<AL, AA>>::RefundActionOutput,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
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
            ))],
            Funded { htlc_location, .. } => {
                vec![alice::ActionKind::Refund(<(AL, AA)>::refund_action(
                    HtlcParams::new_alpha_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                ))]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(alice::ActionKind::Redeem(erc20::redeem_action(
                *htlc_location,
                self.secret_source.secret(),
                request.beta_ledger.network,
            )));
        }
        actions
    }
}
