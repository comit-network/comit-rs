use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, EtherQuantity};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bob::{Accept, Decline},
            Action, StateActions,
        },
        bitcoin,
        ethereum::{self, EtherHtlc, Htlc},
        roles::Bob,
        secret::Secret,
        state_machine::*,
    },
};

impl OngoingSwap<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    pub fn fund_action(&self) -> ethereum::ContractDeploy {
        let htlc = EtherHtlc::from(self.beta_htlc_params());
        let data = htlc.compile_to_hex().into();
        let gas_limit = htlc.deployment_gas_limit();

        ethereum::ContractDeploy {
            data,
            value: self.beta_asset,
            gas_limit,
        }
    }

    pub fn refund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let data = Bytes::default();
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }

    pub fn redeem_action(
        &self,
        beta_htlc_location: OutPoint,
        secret: Secret,
    ) -> bitcoin::SpendOutput {
        let htlc: bitcoin::Htlc = self.alpha_htlc_params().into();

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                beta_htlc_location,
                self.alpha_asset,
                htlc.unlock_with_secret(self.alpha_ledger_success_identity, &secret),
            ),
        }
    }
}

impl StateActions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    type Accept = Accept<Bitcoin, Ethereum>;
    type Decline = Decline<Bitcoin, Ethereum>;
    type Deploy = ();
    type Fund = ethereum::ContractDeploy;
    type Redeem = bitcoin::SpendOutput;
    type Refund = ethereum::SendTransaction;

    fn actions(
        &self,
    ) -> Vec<Action<Self::Accept, Self::Decline, (), Self::Fund, Self::Redeem, Self::Refund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start { ref role, .. }) => vec![
                Action::Accept(role.accept_action()),
                Action::Decline(role.decline_action()),
            ],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => vec![Action::Fund(swap.fund_action())],
            SS::BothFunded(BothFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Refund(swap.refund_action(*beta_htlc_location))],
            SS::AlphaFundedBetaRefunded { .. } => vec![],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Refund(swap.refund_action(*beta_htlc_location))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
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

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin_support;
    use hex::FromHex;
    use swap_protocols::rfc003::{roles::test::Bobisha, Secret};

    #[test]
    fn given_state_instance_when_calling_actions_should_not_need_to_specify_type_arguments() {
        let (bobisha, _) = Bobisha::new();
        let swap_state = SwapStates::from(Start::<Bobisha> {
            alpha_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "875638cac0b0ae9f826575e190f2788918c354c2",
            )
            .unwrap(),
            beta_ledger_success_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger_lock_duration: bitcoin_support::Blocks::from(144),
            secret: Secret::from(*b"hello world, you are beautiful!!").hash(),
            role: bobisha,
        });

        let actions = swap_state.actions();

        assert!(actions
            .into_iter()
            .find(|a| match a {
                Action::Accept(_) => true,
                _ => false,
            })
            .is_some());
    }

}
