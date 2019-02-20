use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum::{self, Erc20Htlc},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Actions, LedgerState, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Token, EtherQuantity};
use std::sync::Arc;

type Request = rfc003::messages::Request<Ethereum, Bitcoin, Erc20Token, BitcoinQuantity>;
type Response = rfc003::messages::AcceptResponseBody<Ethereum, Bitcoin>;

fn fund_action(request: &Request, response: &Response) -> bitcoin::SendToAddress {
    let to = HtlcParams::new_beta_params(request, response).compute_address();
    let amount = request.beta_asset;
    let network = request.beta_ledger.network;

    bitcoin::SendToAddress {
        to,
        amount,
        network,
    }
}

fn _refund_action(
    request: &Request,
    response: &Response,
    beta_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
) -> bitcoin::SpendOutput {
    let beta_asset = request.beta_asset;
    let htlc = bitcoin::Htlc::from(HtlcParams::new_beta_params(request, response));
    let network = request.beta_ledger.network;

    bitcoin::SpendOutput {
        output: PrimedInput::new(
            beta_htlc_location,
            beta_asset,
            htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
        ),
        network,
    }
}

fn redeem_action(
    request: &Request,
    alpha_htlc_location: ethereum_support::Address,
    secret: Secret,
) -> ethereum::SendTransaction {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();
    let network = request.alpha_ledger.network;

    ethereum::SendTransaction {
        to: alpha_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

impl Actions for bob::State<Ethereum, Bitcoin, Erc20Token, BitcoinQuantity> {
    type ActionKind = bob::ActionKind<
        Accept<Ethereum, Bitcoin>,
        Decline<Ethereum, Bitcoin>,
        (),
        bitcoin::SendToAddress,
        ethereum::SendTransaction,
        bitcoin::SpendOutput,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let (request, response) = match &self.swap_communication {
            SwapCommunication::Proposed {
                pending_response, ..
            } => {
                return vec![
                    bob::ActionKind::Accept(Accept::new(
                        pending_response.sender.clone(),
                        Arc::clone(&self.secret_source),
                    )),
                    bob::ActionKind::Decline(Decline::new(pending_response.sender.clone())),
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
        match (alpha_state, beta_state, self.secret) {
            (Funded { htlc_location, .. }, _, Some(secret)) => vec![bob::ActionKind::Redeem(
                redeem_action(&request, *htlc_location, secret),
            )],
            (Funded { .. }, NotDeployed, _) => {
                vec![bob::ActionKind::Fund(fund_action(&request, &response))]
            }
            _ => vec![],
        }
    }
}
