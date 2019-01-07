use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
        },
        ethereum::{self, Erc20Htlc},
        state_machine::*,
        Actions, Bob, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};

impl OngoingSwap<Bob<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    pub fn fund_action(&self) -> bitcoin::SendToAddress {
        bitcoin::SendToAddress {
            address: self.beta_htlc_params().compute_address(),
            value: self.beta_asset,
        }
    }

    pub fn refund_action(&self, beta_htlc_location: OutPoint) -> bitcoin::SpendOutput {
        bitcoin::SpendOutput {
            output: PrimedInput::new(
                beta_htlc_location,
                self.beta_asset,
                bitcoin::Htlc::from(self.beta_htlc_params())
                    .unlock_after_timeout(self.beta_ledger_refund_identity),
            ),
        }
    }

    pub fn redeem_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
        secret: Secret,
    ) -> ethereum::SendTransaction {
        let data = Bytes::from(secret.raw_secret().to_vec());
        let gas_limit = Erc20Htlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }
}

impl Actions for SwapStates<Bob<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    type ActionKind = bob::ActionKind<
        Accept<Ethereum, Bitcoin>,
        Decline<Ethereum, Bitcoin>,
        (),
        bitcoin::SendToAddress,
        ethereum::SendTransaction,
        bitcoin::SpendOutput,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start { ref role, .. }) => vec![
                bob::ActionKind::Accept(role.accept_action()),
                bob::ActionKind::Decline(role.decline_action()),
            ],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                vec![bob::ActionKind::Fund(swap.fund_action())]
            }
            SS::BothFunded(BothFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![bob::ActionKind::Refund(
                swap.refund_action(*beta_htlc_location),
            )],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref beta_redeemed_tx,
                ..
            }) => vec![bob::ActionKind::Redeem(
                swap.redeem_action(*alpha_htlc_location, beta_redeemed_tx.secret),
            )],
            _ => vec![],
        }
    }
}
