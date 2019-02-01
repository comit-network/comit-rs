use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        alice::{self, SwapCommunication},
        bitcoin,
        ethereum::{self, EtherHtlc},
        secret::Secret,
        secret_source::SecretSource,
        swap_accepted, Actions, LedgerState,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, EtherQuantity};

type SwapAccepted = swap_accepted::SwapAccepted<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>;

pub fn fund_action(swap_accepted: &SwapAccepted) -> ethereum::ContractDeploy {
    swap_accepted.alpha_htlc_params().into()
}

pub fn _refund_action(
    swap_accepted: &SwapAccepted,
    alpha_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = EtherHtlc::tx_gas_limit();
    let network = swap_accepted.request.alpha_ledger.network;

    ethereum::SendTransaction {
        to: alpha_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

pub fn redeem_action(
    swap_accepted: &SwapAccepted,
    beta_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
    secret: Secret,
) -> bitcoin::SpendOutput {
    let beta_asset = swap_accepted.request.beta_asset;
    let htlc = bitcoin::Htlc::from(swap_accepted.beta_htlc_params());
    let network = swap_accepted.request.beta_ledger.network;

    bitcoin::SpendOutput {
        output: PrimedInput::new(
            beta_htlc_location,
            beta_asset,
            htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
        ),
        network,
    }
}

impl Actions for alice::State<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity> {
    type ActionKind = alice::ActionKind<
        (),
        ethereum::ContractDeploy,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let swap_accepted = match self.swap_communication {
            SwapCommunication::Accepted { ref swap_accepted } => swap_accepted,
            SwapCommunication::Proposed { .. } | SwapCommunication::Rejected { .. } => {
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
                self.secret_source.as_ref(),
                self.secret_source.secret(),
            ))],
            (NotDeployed, NotDeployed) => {
                vec![alice::ActionKind::Fund(fund_action(&swap_accepted))]
            }
            _ => vec![],
        }
    }
}
