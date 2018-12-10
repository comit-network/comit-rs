use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, EtherQuantity, U256};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{Action, Actions},
        bitcoin,
        ethereum::{self, EtherHtlc},
        roles::Alice,
        state_machine::*,
    },
};

impl OngoingSwap<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    pub fn fund_action(&self) -> bitcoin::SendToAddress {
        bitcoin::SendToAddress {
            address: self.alpha_htlc_params().compute_address(),
            value: self.alpha_asset,
        }
    }

    pub fn refund_action(&self, alpha_htlc_location: OutPoint) -> bitcoin::SpendOutput {
        bitcoin::SpendOutput {
            output: PrimedInput::new(
                alpha_htlc_location,
                self.alpha_asset,
                bitcoin::Htlc::from(self.alpha_htlc_params())
                    .unlock_after_timeout(self.alpha_ledger_refund_identity),
            ),
        }
    }

    pub fn redeem_action(
        &self,
        beta_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let data = Bytes::from(self.secret.raw_secret().to_vec());
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: beta_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::from_wei(U256::zero()),
        }
    }
}

type AliceActionKind =
    Action<(), (), (), bitcoin::SendToAddress, ethereum::SendTransaction, bitcoin::SpendOutput>;

impl Actions for SwapStates<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    type ActionKind = AliceActionKind;

    fn actions(&self) -> Vec<AliceActionKind> {
        use self::SwapStates as SS;
        match *self {
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Fund(swap.fund_action())],
            SS::BothFunded(BothFunded {
                ref alpha_htlc_location,
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![
                Action::Redeem(swap.redeem_action(*beta_htlc_location)),
                Action::Refund(swap.refund_action(*alpha_htlc_location)),
            ],
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                ref swap,
                ref alpha_htlc_location,
                ..
            })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ..
            }) => vec![Action::Refund(swap.refund_action(*alpha_htlc_location))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Redeem(swap.redeem_action(*beta_htlc_location))],
            _ => vec![],
        }
    }
}
