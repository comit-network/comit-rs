use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum::{self, Erc20Htlc},
        secret_source::SecretSource,
        swap_accepted, Actions, LedgerState, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Token, EtherQuantity};
use std::sync::Arc;

type SwapAccepted = swap_accepted::SwapAccepted<Ethereum, Bitcoin, Erc20Token, BitcoinQuantity>;

fn fund_action(swap_accepted: &SwapAccepted) -> bitcoin::SendToAddress {
    let to = swap_accepted.beta_htlc_params().compute_address();
    let amount = swap_accepted.request.beta_asset;
    let network = swap_accepted.request.beta_ledger.network;

    bitcoin::SendToAddress {
        to,
        amount,
        network,
    }
}

fn _refund_action(
    swap_accepted: &SwapAccepted,
    beta_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
) -> bitcoin::SpendOutput {
    let beta_asset = swap_accepted.request.beta_asset;
    let htlc = bitcoin::Htlc::from(swap_accepted.beta_htlc_params());
    let network = swap_accepted.request.beta_ledger.network;

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
    swap_accepted: &SwapAccepted,
    alpha_htlc_location: ethereum_support::Address,
    secret: Secret,
) -> ethereum::SendTransaction {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();
    let network = swap_accepted.request.alpha_ledger.network;

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
        dbg!(self.clone());
        let swap_accepted = match &self.swap_communication {
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
            SwapCommunication::Accepted { ref swap_accepted } => swap_accepted,
            SwapCommunication::Rejected { .. } => return vec![],
        };

        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;
        // dbg!((alpha_state.clone(), beta_state.clone(), self.secret.clone()));
        match (alpha_state, beta_state, self.secret) {
            (Funded { htlc_location, .. }, _, Some(secret)) => vec![bob::ActionKind::Redeem(
                redeem_action(&swap_accepted, *htlc_location, secret),
            )],
            (Funded { .. }, NotDeployed, _) => {
                vec![bob::ActionKind::Fund(fund_action(&swap_accepted))]
            }
            _ => vec![],
        }
    }
}
