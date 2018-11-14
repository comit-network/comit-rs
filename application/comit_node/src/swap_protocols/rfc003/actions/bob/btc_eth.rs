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
        bitcoin::bitcoin_htlc,
        ethereum::ethereum_htlc,
        state_machine::*,
        SecretHash,
    },
};

impl StateActions for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash> {
    type Accept = Accept;
    type Decline = Decline;
    type Fund = EtherDeploy;
    type Redeem = BitcoinRedeem;
    type Refund = EtherRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, EtherDeploy, BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::Accepted { .. } => vec![],
            SS::SourceFunded(SourceFunded { ref swap, .. }) => {
                let htlc = ethereum_htlc(swap);
                vec![Action::Fund(EtherDeploy {
                    data: htlc.compile_to_hex().into(),
                    value: swap.target_asset,
                    gas_limit: 42.into(), //TODO come up with correct gas limit
                    gas_cost: 42.into(),  //TODO come up with correct gas cost
                })]
            }
            SS::BothFunded(BothFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceFundedTargetRefunded { .. } => vec![],
            SS::SourceFundedTargetRedeemed { .. } => vec![],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref swap,
                ref target_htlc_location,
                ref source_htlc_location,
                ref secret,
            }) => vec![
                Action::Redeem(BitcoinRedeem {
                    outpoint: *source_htlc_location,
                    htlc: bitcoin_htlc(swap),
                    value: swap.source_asset,
                    transient_keypair: swap.source_ledger_refund_identity,
                    secret: *secret,
                }),
                Action::Refund(EtherRefund {
                    contract_address: *target_htlc_location,
                    gas_limit: 42.into(), //TODO come up with correct gas_limit
                    gas_cost: 42.into(),  //TODO come up with correct gas cost
                }),
            ],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin_support;
    use hex;
    use secp256k1_support;
    use swap_protocols::rfc003::Secret;

    #[test]
    fn given_state_instance_when_calling_actions_should_not_need_to_specify_type_arguments() {
        let swap_state = SwapStates::from(Start {
            source_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap(),
            target_ledger_success_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
            source_asset: BitcoinQuantity::from_bitcoin(1.0),
            target_asset: EtherQuantity::from_eth(10.0),
            source_ledger_lock_duration: bitcoin_support::Blocks::from(144),
            secret: Secret::from(*b"hello world, you are beautiful!!").hash(),
        });

        let actions = swap_state.actions();

        assert_eq!(
            actions,
            vec![Action::Accept(Accept), Action::Decline(Decline)]
        );
    }

}
