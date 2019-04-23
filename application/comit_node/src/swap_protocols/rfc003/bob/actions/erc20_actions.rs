use crate::swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{
        actions::{non_erc20::CreateActions, Actions},
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum::{self, Erc20Htlc},
        state_machine::HtlcParams,
        Action, Ledger, LedgerState, Secret,
    },
};
use ethereum_support::{Bytes, Erc20Token, EtherQuantity, Network};
use std::sync::Arc;

fn deploy_action(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> ethereum::ContractDeploy {
    htlc_params.into()
}

pub fn fund_action(
    htlc_params: HtlcParams<Ethereum, Erc20Token>,
    to_erc20_contract: ethereum_support::Address,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let htlc = Erc20Htlc::from(htlc_params.clone());
    let gas_limit = Erc20Htlc::fund_tx_gas_limit();
    let network = htlc_params.ledger.network;

    ethereum::SendTransaction {
        to: to_erc20_contract,
        data: htlc.funding_tx_payload(beta_htlc_location),
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

pub fn refund_action(
    network: Network,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::SendTransaction {
        to: beta_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

fn redeem_action(
    alpha_htlc_location: ethereum_support::Address,
    secret: Secret,
    network: Network,
) -> ethereum::SendTransaction {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::SendTransaction {
        to: alpha_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

impl<AL, AA> Actions for bob::State<AL, Ethereum, AA, Erc20Token>
where
    AL: Ledger,
    AA: Asset,
    (AL, AA): CreateActions<AL, AA>,
{
    type ActionKind = bob::ActionKind<
        Accept<AL, Ethereum>,
        Decline<AL, Ethereum>,
        ethereum::ContractDeploy,
        ethereum::SendTransaction,
        <(AL, AA) as CreateActions<AL, AA>>::RedeemActionOutput,
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
                vec![bob::ActionKind::Redeem(<(AL, AA)>::redeem_action(
                    HtlcParams::new_alpha_params(request, response),
                    htlc_location.clone(),
                    &*self.secret_source,
                    secret,
                ))
                .into_action()]
            }
            (Funded { .. }, NotDeployed, _) => {
                vec![
                    bob::ActionKind::Deploy(deploy_action(HtlcParams::new_beta_params(
                        request, response,
                    )))
                    .into_action(),
                ]
            }
            (Funded { .. }, Deployed { htlc_location, .. }, _) => {
                vec![bob::ActionKind::Fund(fund_action(
                    HtlcParams::new_beta_params(request, response),
                    request.beta_asset.token_contract,
                    *htlc_location,
                ))
                .into_action()]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                bob::ActionKind::Refund(refund_action(request.beta_ledger.network, *htlc_location))
                    .into_action()
                    .with_invalid_until(request.beta_expiry),
            );
        }
        actions
    }
}

impl<BL, BA> Actions for bob::State<Ethereum, BL, Erc20Token, BA>
where
    BL: Ledger,
    BA: Asset,
    (BL, BA): CreateActions<BL, BA>,
{
    type ActionKind = bob::ActionKind<
        Accept<Ethereum, BL>,
        Decline<Ethereum, BL>,
        (),
        <(BL, BA) as CreateActions<BL, BA>>::FundActionOutput,
        ethereum::SendTransaction,
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
                (Funded { htlc_location, .. }, _, Some(secret)) => vec![bob::ActionKind::Redeem(
                    redeem_action(*htlc_location, secret, request.alpha_ledger.network),
                )
                .into_action()],
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
