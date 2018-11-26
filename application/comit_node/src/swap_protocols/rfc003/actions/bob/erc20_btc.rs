use bitcoin_support::BitcoinQuantity;
use ethereum_support::Erc20Quantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::{BitcoinFund, BitcoinRefund},
            ethereum::Erc20Redeem,
            Accept, Action, Decline, StateActions,
        },
        ethereum::{Erc20Htlc, Htlc},
        roles::Bob,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Bob<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    type Accept = Accept;
    type Decline = Decline;
    type Deploy = ();
    type Fund = BitcoinFund;
    type Redeem = Erc20Redeem;
    type Refund = BitcoinRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, (), BitcoinFund, Erc20Redeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::Accepted { .. } => vec![],
            SS::AlphaDeployed { .. } => vec![],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                let htlc: Erc20Htlc = swap.beta_htlc_params().into();
                vec![Action::Fund(BitcoinFund {
                    data: htlc.compile_to_hex().into(),
                    value: swap.beta_asset,
                    gas_limit: 42.into(), //TODO come up with correct gas limit
                    gas_cost: 42.into(),  //TODO come up with correct gas cost
                })]
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { .. }) => vec![], // TODO: Return Beta Funding action
            SS::BothFunded(BothFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                contract_address: *beta_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::AlphaFundedBetaRefunded { .. } => vec![],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                contract_address: *beta_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                contract_address: *beta_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(Erc20Redeem {
                outpoint: *alpha_htlc_location,
                htlc: swap.alpha_htlc_params().into(),
                value: swap.alpha_asset,
                transient_keypair: swap.alpha_ledger_success_identity,
                secret: *secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
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
            alpha_ledger: Ethereum::default(),
            beta_ledger: Bitcoin::default(),
            alpha_asset: Erc20Quantity::from_bitcoin(1.0),
            beta_asset: BitcoinQuantity::from_eth(10.0),
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
