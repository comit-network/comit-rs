use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self,
        actions::CreateActions,
        bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum::{self, Erc20Htlc},
        state_machine::HtlcParams,
        Action, Actions, LedgerState,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Bytes, Erc20Token, EtherQuantity};
use std::sync::Arc;

type Request = rfc003::messages::Request<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token>;
type Response = rfc003::messages::AcceptResponseBody<Bitcoin, Ethereum>;

fn deploy_action(request: &Request, response: &Response) -> ethereum::ContractDeploy {
    HtlcParams::new_beta_params(request, response).into()
}

pub fn fund_action(
    request: &Request,
    response: &Response,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let to = request.beta_asset.token_contract;
    let htlc = Erc20Htlc::from(HtlcParams::new_beta_params(request, response));
    let gas_limit = Erc20Htlc::fund_tx_gas_limit();
    let network = request.beta_ledger.network;

    ethereum::SendTransaction {
        to,
        data: htlc.funding_tx_payload(beta_htlc_location),
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

pub fn refund_action(
    request: &Request,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = Erc20Htlc::tx_gas_limit();
    let network = request.beta_ledger.network;

    ethereum::SendTransaction {
        to: beta_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

impl Actions for bob::State<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token> {
    type ActionKind = bob::ActionKind<
        Accept<Bitcoin, Ethereum>,
        Decline<Bitcoin, Ethereum>,
        ethereum::ContractDeploy,
        ethereum::SendTransaction,
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
            (Funded { htlc_location, .. }, _, Some(_secret)) => {
                vec![
                    bob::ActionKind::Redeem(<(Bitcoin, BitcoinQuantity)>::redeem_action(
                        HtlcParams::new_alpha_params(request, response),
                        htlc_location.clone(),
                        &*self.secret_source,
                    ))
                    .into_action(),
                ]
            }
            (Funded { .. }, NotDeployed, _) => {
                vec![bob::ActionKind::Deploy(deploy_action(&request, &response)).into_action()]
            }
            (Funded { .. }, Deployed { htlc_location, .. }, _) => {
                vec![
                    bob::ActionKind::Fund(fund_action(&request, &response, *htlc_location))
                        .into_action(),
                ]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                bob::ActionKind::Refund(refund_action(&request, *htlc_location))
                    .into_action()
                    .with_invalid_until(request.beta_expiry),
            );
        }
        actions
    }
}
