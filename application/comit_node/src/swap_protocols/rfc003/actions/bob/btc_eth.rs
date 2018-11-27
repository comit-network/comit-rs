use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::BitcoinRedeem,
            ethereum::{EtherDeploy, EtherRefund},
            Accept, Action, Decline, StateActions,
        },
        ethereum::{EtherHtlc, Htlc},
        roles::Bob,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    type Accept = Accept;
    type Decline = Decline;
    type Deploy = EtherDeploy;
    type Fund = ();
    type Redeem = BitcoinRedeem;
    type Refund = EtherRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, EtherDeploy, (), BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                let htlc: EtherHtlc = swap.beta_htlc_params().into();
                vec![Action::Deploy(EtherDeploy::new(
                    htlc.compile_to_hex().into(),
                    swap.beta_asset,
                ))]
            }
            SS::BothFunded(BothFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund::new(*beta_htlc_location))],
            SS::AlphaFundedBetaRefunded { .. } => vec![],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund::new(*beta_htlc_location))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund::new(*beta_htlc_location))],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(BitcoinRedeem::new(
                *alpha_htlc_location,
                swap.alpha_htlc_params().into(),
                swap.alpha_asset,
                swap.alpha_ledger_success_identity,
                *secret,
            ))],
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
