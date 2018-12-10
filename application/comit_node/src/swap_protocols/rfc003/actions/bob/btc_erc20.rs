use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bob::{Accept, Decline},
            Action, StateActions,
        },
        bitcoin,
        ethereum::{self, Erc20Htlc, Htlc},
        roles::Bob,
        secret::Secret,
        state_machine::*,
    },
};

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
        beta_htlc_location: OutPoint,
        secret: Secret,
    ) -> bitcoin::SpendOutput {
        bitcoin::SpendOutput {
            output: PrimedInput::new(
                beta_htlc_location,
                self.alpha_asset,
                bitcoin::Htlc::from(self.alpha_htlc_params())
                    .unlock_with_secret(self.alpha_ledger_redeem_identity, &secret),
            ),
        }
    }
}

impl StateActions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> {
    type Accept = Accept<Bitcoin, Ethereum>;
    type Decline = Decline<Bitcoin, Ethereum>;
    type Deploy = ethereum::ContractDeploy;
    type Fund = ethereum::SendTransaction;
    type Redeem = bitcoin::SpendOutput;
    type Refund = ethereum::SendTransaction;

    #[allow(clippy::type_complexity)]
    fn actions(
        &self,
    ) -> Vec<
        Action<Self::Accept, Self::Decline, Self::Deploy, Self::Fund, Self::Redeem, Self::Refund>,
    > {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start { ref role, .. }) => vec![
                Action::Accept(role.accept_action()),
                Action::Decline(role.decline_action()),
            ],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                vec![Action::Deploy(swap.deploy_action())]
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Fund(swap.fund_action(*beta_htlc_location))],
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
            }) => vec![Action::Refund(swap.refund_action(*beta_htlc_location))],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref beta_redeemed_tx,
                ..
            }) => vec![Action::Redeem(
                swap.redeem_action(*alpha_htlc_location, beta_redeemed_tx.secret),
            )],
            _ => vec![],
        }
    }
}
