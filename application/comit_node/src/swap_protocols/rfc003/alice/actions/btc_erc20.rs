use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        alice::{self, SwapCommunication},
        bitcoin,
        ethereum::{self, Erc20Htlc},
        secret::Secret,
        secret_source::SecretSource,
        swap_accepted, Actions, LedgerState,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Token, EtherQuantity};

type SwapAccepted = swap_accepted::SwapAccepted<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token>;

fn fund_action(swap_accepted: &SwapAccepted) -> bitcoin::SendToAddress {
    let to = swap_accepted.alpha_htlc_params().compute_address();
    let amount = swap_accepted.request.alpha_asset;
    let network = swap_accepted.request.alpha_ledger.network;

    bitcoin::SendToAddress {
        to,
        amount,
        network,
    }
}

fn _refund_action(
    swap_accepted: &SwapAccepted,
    alpha_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
) -> bitcoin::SpendOutput {
    let alpha_asset = swap_accepted.request.alpha_asset;
    let htlc = bitcoin::Htlc::from(swap_accepted.alpha_htlc_params());
    let network = swap_accepted.request.alpha_ledger.network;

    bitcoin::SpendOutput {
        output: PrimedInput::new(
            alpha_htlc_location,
            alpha_asset,
            htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
        ),
        network,
    }
}

fn redeem_action(
    swap_accepted: &SwapAccepted,
    beta_htlc_location: ethereum_support::Address,
    secret: Secret,
) -> ethereum::SendTransaction {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();
    let network = swap_accepted.request.beta_ledger.network;

    ethereum::SendTransaction {
        to: beta_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

impl Actions for alice::State<Bitcoin, Ethereum, BitcoinQuantity, Erc20Token> {
    type ActionKind = alice::ActionKind<
        (),
        bitcoin::SendToAddress,
        ethereum::SendTransaction,
        bitcoin::SpendOutput,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let swap_accepted = match self.swap_communication {
            SwapCommunication::Accepted { ref swap_accepted } => swap_accepted,
            _ => {
                return vec![];
            }
        };
        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;
        match (alpha_state, beta_state) {
            (_, Funded { htlc_location, .. }) => vec![alice::ActionKind::Redeem(redeem_action(
                &swap_accepted,
                *htlc_location,
                self.secret_source.secret(),
            ))],
            (NotDeployed, NotDeployed) => {
                vec![alice::ActionKind::Fund(fund_action(&swap_accepted))]
            }
            _ => vec![],
        }
    }
}
