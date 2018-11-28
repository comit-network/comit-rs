use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Bytes, EtherQuantity};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{Accept, Action, Decline, StateActions},
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
        ethereum::ContractDeploy {
            data: htlc.compile_to_hex().into(),
            value: self.beta_asset,
            gas_limit: 420_000.into(), //TODO: Calculate properly
        }
    }
    pub fn refund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data: Bytes::default(),
            gas_limit: 42_000.into(), //TODO: Calculate properly
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
    type Accept = Accept;
    type Decline = Decline;
    type Deploy = ();
    type Fund = ethereum::ContractDeploy;
    type Redeem = bitcoin::SpendOutput;
    type Refund = ethereum::SendTransaction;

    fn actions(
        &self,
    ) -> Vec<
        Action<
            Accept,
            Decline,
            (),
            ethereum::ContractDeploy,
            bitcoin::SpendOutput,
            ethereum::SendTransaction,
        >,
    > {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
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
                ref secret,
                ..
            }) => vec![Action::Redeem(
                swap.redeem_action(*alpha_htlc_location, *secret),
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
        });

        let actions = swap_state.actions();

        assert_eq!(
            actions,
            vec![Action::Accept(Accept), Action::Decline(Decline)]
        );
    }

}
