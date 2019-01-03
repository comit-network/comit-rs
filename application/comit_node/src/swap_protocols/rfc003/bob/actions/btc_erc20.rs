use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
        },
        ethereum::{self, Erc20Htlc, Htlc},
        secret::Secret,
        state_machine::*,
        Actions, Bob,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};

impl OngoingSwap<Bob<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> {
    pub fn deploy_action(&self) -> ethereum::ContractDeploy {
        let htlc = Erc20Htlc::from(self.beta_htlc_params());
        let data = htlc.compile_to_hex().into();
        let gas_limit = htlc.deployment_gas_limit();

        ethereum::ContractDeploy {
            data,
            value: EtherQuantity::zero(),
            gas_limit,
        }
    }

    pub fn refund_action(
        &self,
        beta_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let data = Bytes::default();
        let gas_limit = Erc20Htlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: beta_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }

    pub fn fund_action(
        &self,
        beta_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let htlc = Erc20Htlc::from(self.beta_htlc_params());
        let gas_limit = Erc20Htlc::fund_tx_gas_limit();

        ethereum::SendTransaction {
            to: self.beta_asset.token_contract(),
            data: htlc.funding_tx_payload(beta_htlc_location),
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }

    pub fn redeem_action(
        &self,
        alpha_htlc_location: OutPoint,
        secret: Secret,
    ) -> bitcoin::SpendOutput {
        bitcoin::SpendOutput {
            output: PrimedInput::new(
                alpha_htlc_location,
                self.alpha_asset,
                bitcoin::Htlc::from(self.alpha_htlc_params())
                    .unlock_with_secret(self.alpha_ledger_redeem_identity, &secret),
            ),
        }
    }
}

impl Actions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> {
    type ActionKind = bob::ActionKind<
        Accept<Bitcoin, Ethereum>,
        Decline<Bitcoin, Ethereum>,
        ethereum::ContractDeploy,
        ethereum::SendTransaction,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start { ref role, .. }) => vec![
                bob::ActionKind::Accept(role.accept_action()),
                bob::ActionKind::Decline(role.decline_action()),
            ],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                vec![bob::ActionKind::Deploy(swap.deploy_action())]
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![bob::ActionKind::Fund(swap.fund_action(*beta_htlc_location))],
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
