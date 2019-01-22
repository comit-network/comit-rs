use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        alice, bitcoin,
        ethereum::{self, Erc20Htlc, Htlc},
        state_machine::*,
        Actions, Alice,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};

impl OngoingSwap<Alice<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    pub fn deploy_action(&self) -> ethereum::ContractDeploy {
        let htlc = Erc20Htlc::from(self.alpha_htlc_params());
        let data = htlc.compile_to_hex().into();
        let gas_limit = htlc.deployment_gas_limit();

        ethereum::ContractDeploy {
            data,
            amount: EtherQuantity::zero(),
            gas_limit,
            network: self.alpha_ledger.network,
        }
    }

    pub fn fund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let htlc = Erc20Htlc::from(self.alpha_htlc_params());
        let gas_limit = Erc20Htlc::fund_tx_gas_limit();

        ethereum::SendTransaction {
            to: self.alpha_asset.token_contract(),
            data: htlc.funding_tx_payload(alpha_htlc_location),
            gas_limit,
            amount: EtherQuantity::zero(),
            network: self.alpha_ledger.network,
        }
    }

    pub fn refund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let data = Bytes::default();
        let gas_limit = Erc20Htlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            amount: EtherQuantity::zero(),
            network: self.alpha_ledger.network,
        }
    }

    pub fn redeem_action(&self, beta_htlc_location: OutPoint) -> bitcoin::SpendOutput {
        let htlc: bitcoin::Htlc = self.beta_htlc_params().into();

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                beta_htlc_location,
                self.beta_asset,
                htlc.unlock_with_secret(self.beta_ledger_redeem_identity, &self.secret),
            ),
            network: self.beta_ledger.network,
        }
    }
}

impl Actions for SwapStates<Alice<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    type ActionKind = alice::ActionKind<
        ethereum::ContractDeploy,
        ethereum::SendTransaction,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        use self::SwapStates as SS;
        match *self {
            SS::Accepted(Accepted { ref swap, .. }) => {
                vec![alice::ActionKind::Deploy(swap.deploy_action())]
            }
            SS::AlphaDeployed(AlphaDeployed {
                ref swap,
                ref alpha_htlc_location,
                ..
            }) => vec![alice::ActionKind::Fund(
                swap.fund_action(*alpha_htlc_location),
            )],
            SS::BothFunded(BothFunded {
                ref alpha_htlc_location,
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![
                alice::ActionKind::Redeem(swap.redeem_action(*beta_htlc_location)),
                alice::ActionKind::Refund(swap.refund_action(*alpha_htlc_location)),
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
            }) => vec![alice::ActionKind::Refund(
                swap.refund_action(*alpha_htlc_location),
            )],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![alice::ActionKind::Redeem(
                swap.redeem_action(*beta_htlc_location),
            )],
            _ => vec![],
        }
    }
}
