use super::{
    AcceptRequest, Action, BitcoinRedeem, DeclineRequest, EtherDeploy, EtherRefund, StateActions,
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{bitcoin::bitcoin_htlc, ethereum::ethereum_htlc, state_machine::*, SecretHash},
};

impl StateActions<AcceptRequest, DeclineRequest, EtherDeploy, BitcoinRedeem, EtherRefund>
    for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash>
{
    fn actions(
        &self,
    ) -> Vec<Action<AcceptRequest, DeclineRequest, EtherDeploy, BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![
                Action::Accept(AcceptRequest),
                Action::Decline(DeclineRequest),
            ],
            SS::Accepted { .. } => vec![],
            SS::SourceFunded(SourceFunded { ref swap, .. }) => {
                let htlc = ethereum_htlc(swap);
                vec![Action::FundHtlc(EtherDeploy {
                    data: htlc.compile_to_hex().into(),
                    value: swap.target_asset,
                    gas_limit: 42, //TODO come up with correct gas_limit
                })]
            }
            SS::BothFunded(BothFunded {
                ref target_htlc_id, ..
            }) => vec![Action::RefundHtlc(EtherRefund {
                contract_address: target_htlc_id.clone(),
                execution_gas: 42, //TODO: generate gas cost directly
            })],
            SS::SourceFundedTargetRefunded { .. } => vec![],
            SS::SourceFundedTargetRedeemed { .. } => vec![],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_id,
                ..
            }) => vec![Action::RefundHtlc(EtherRefund {
                contract_address: target_htlc_id.clone(),
                execution_gas: 42, //TODO: generate gas cost directly
            })],
            SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref swap,
                ref target_htlc_id,
                ref source_htlc_id,
                ref secret,
            }) => vec![
                Action::RedeemHtlc(BitcoinRedeem {
                    outpoint: source_htlc_id.clone(),
                    htlc: bitcoin_htlc(swap),
                    value: swap.source_asset,
                    transient_keypair: swap.source_identity.into(),
                    secret: *secret,
                }),
                Action::RefundHtlc(EtherRefund {
                    contract_address: target_htlc_id.clone(),
                    execution_gas: 42, //TODO: generate gas cost directly
                }),
            ],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
