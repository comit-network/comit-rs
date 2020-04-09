use crate::{
    asset::{self},
    htlc_location, identity,
    swap_protocols::{
        actions::{ethereum, Actions},
        ledger::Ethereum,
        rfc003::{
            actions::{
                erc20, Accept, Action, Decline, MakeFundAction, MakeRedeemAction, MakeRefundAction,
            },
            alice,
            create_swap::HtlcParams,
            DeriveSecret, LedgerState, SwapCommunication,
        },
    },
    transaction,
};
use std::convert::Infallible;

impl<BL, BA, BH, BI, BT> Actions
    for alice::State<
        Ethereum,
        BL,
        asset::Erc20,
        BA,
        htlc_location::Ethereum,
        BH,
        identity::Ethereum,
        BI,
        transaction::Ethereum,
        BT,
    >
where
    BL: Clone,
    BA: Clone,
    BH: Clone,
    BI: Clone,
    BT: Clone,
    (BL, BA): MakeRedeemAction<HtlcParams = HtlcParams<BL, BA, BI>, HtlcLocation = BH>,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<Ethereum, BL>,
        Decline<Ethereum, BL>,
        ethereum::DeployContract,
        ethereum::CallContract,
        <(BL, BA) as MakeRedeemAction>::Output,
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
            NotDeployed => vec![Action::Deploy(erc20::deploy_action(
                HtlcParams::new_alpha_params(request, response),
            ))],
            Deployed { htlc_location, .. } => vec![Action::Fund(erc20::fund_action(
                HtlcParams::new_alpha_params(request, response),
                request.alpha_asset.token_contract,
                *htlc_location,
            ))],
            Funded { htlc_location, .. } => vec![Action::Refund(erc20::refund_action(
                request.alpha_ledger.chain_id,
                request.alpha_expiry,
                *htlc_location,
            ))],
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(Action::Redeem(<(BL, BA)>::make_redeem_action(
                HtlcParams::new_beta_params(request, response),
                htlc_location.clone(),
                &self.secret_source, // Derive identities with this.
                self.secret_source.derive_secret(), // The secret used by Alice.
            )));
        }
        actions
    }
}

impl<AL, AA, AH, AI, AT> Actions
    for alice::State<
        AL,
        Ethereum,
        AA,
        asset::Erc20,
        AH,
        htlc_location::Ethereum,
        AI,
        identity::Ethereum,
        AT,
        transaction::Ethereum,
    >
where
    AL: Clone,
    AA: Clone,
    AH: Copy,
    AI: Clone,
    AT: Clone,
    (AL, AA): MakeFundAction<HtlcParams = HtlcParams<AL, AA, AI>>
        + MakeRefundAction<
            HtlcParams = HtlcParams<AL, AA, AI>,
            HtlcLocation = AH,
            FundTransaction = AT,
        >,
{
    #[allow(clippy::type_complexity)]
    type ActionKind = Action<
        Accept<AL, Ethereum>,
        Decline<AL, Ethereum>,
        Infallible,
        <(AL, AA) as MakeFundAction>::Output,
        ethereum::CallContract,
        <(AL, AA) as MakeRefundAction>::Output,
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
            NotDeployed => vec![Action::Fund(<(AL, AA)>::make_fund_action(
                HtlcParams::new_alpha_params(request, response),
            ))],
            Funded {
                htlc_location,
                fund_transaction,
                ..
            } => vec![Action::Refund(<(AL, AA)>::make_refund_action(
                HtlcParams::new_alpha_params(request, response),
                *htlc_location,
                &self.secret_source,
                fund_transaction,
            ))],
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(Action::Redeem(erc20::redeem_action(
                *htlc_location,
                self.secret_source.derive_secret(),
                request.beta_ledger.chain_id,
            )));
        }
        actions
    }
}
