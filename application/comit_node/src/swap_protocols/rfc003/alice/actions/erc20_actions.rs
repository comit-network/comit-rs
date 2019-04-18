use crate::swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        alice::{self, actions::CreateActions, SwapCommunication},
        ethereum::{self, Erc20Htlc},
        secret::Secret,
        state_machine::HtlcParams,
        Action, Actions, Ledger, LedgerState,
    },
};
use ethereum_support::{Bytes, Erc20Token, EtherQuantity, Network};

fn deploy_action(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> ethereum::ContractDeploy {
    htlc_params.into()
}

fn fund_action(
    htlc_params: HtlcParams<Ethereum, Erc20Token>,
    to_erc20_contract: ethereum_support::Address,
    htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let htlc = Erc20Htlc::from(htlc_params.clone());
    let gas_limit = Erc20Htlc::fund_tx_gas_limit();
    let network = htlc_params.ledger.network;

    ethereum::SendTransaction {
        to: to_erc20_contract,
        data: htlc.funding_tx_payload(htlc_location),
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

fn refund_action(
    network: Network,
    alpha_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::SendTransaction {
        to: alpha_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

fn redeem_action(
    network: Network,
    beta_htlc_location: ethereum_support::Address,
    secret: Secret,
) -> ethereum::SendTransaction {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::SendTransaction {
        to: beta_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

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

        let mut actions = match alpha_state {
            NotDeployed => {
                vec![
                    alice::ActionKind::Deploy(deploy_action(HtlcParams::new_alpha_params(
                        request, response,
                    )))
                    .into_action(),
                ]
            }
            Deployed { htlc_location, .. } => vec![alice::ActionKind::Fund(fund_action(
                HtlcParams::new_alpha_params(request, response),
                request.alpha_asset.token_contract,
                *htlc_location,
            ))
            .into_action()],
            Funded { htlc_location, .. } => vec![alice::ActionKind::Refund(refund_action(
                request.alpha_ledger.network,
                *htlc_location,
            ))
            .into_action()
            .with_invalid_until(request.alpha_expiry)],
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                alice::ActionKind::Redeem(<(BL, BA)>::redeem_action(
                    request.beta_asset,
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
                alice::ActionKind::Redeem(redeem_action(
                    request.beta_ledger.network,
                    *htlc_location,
                    self.secret_source.secret(),
                ))
                .into_action(),
            );
        }
        actions
    }
}
