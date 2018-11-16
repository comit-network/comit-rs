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
    type Fund = EtherDeploy;
    type Redeem = BitcoinRedeem;
    type Refund = EtherRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, EtherDeploy, BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::Accepted { .. } => vec![],
            SS::SourceFunded(SourceFunded { ref swap, .. }) => {
                let htlc: EtherHtlc = swap.target_htlc_params().into();
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
            SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref swap,
                ref source_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(BitcoinRedeem {
                outpoint: *source_htlc_location,
                htlc: swap.source_htlc_params().into(),
                value: swap.source_asset,
                transient_keypair: swap.source_ledger_success_identity,
                secret: *secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
