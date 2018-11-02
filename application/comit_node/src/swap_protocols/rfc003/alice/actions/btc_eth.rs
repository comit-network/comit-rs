use super::{Action, BitcoinFund, BitcoinRefund, EtherRedeem, StateActions};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin::{bitcoin_htlc, bitcoin_htlc_address},
        state_machine::*,
        Secret,
    },
};

impl StateActions<BitcoinFund, EtherRedeem, BitcoinRefund>
    for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, Secret>
{
    fn actions(&self) -> Vec<Action<BitcoinFund, EtherRedeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![],
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::FundHtlc(BitcoinFund {
                address: bitcoin_htlc_address(swap),
                value: swap.source_asset,
            })],
            SS::SourceFunded { .. } => vec![],
            SS::BothFunded(BothFunded {
                ref source_htlc_id,
                ref target_htlc_id,
                ref swap,
                ..
            }) => vec![
                Action::RedeemHtlc(EtherRedeem {
                    contract_address: target_htlc_id.clone(),
                    execution_gas: 42, //TODO: generate gas cost directly
                    data: swap.secret,
                }),
                Action::RefundHtlc(BitcoinRefund {
                    outpoint: source_htlc_id.clone(),
                    htlc: bitcoin_htlc(swap),
                    value: swap.source_asset,
                    transient_keypair: swap.source_identity.into(),
                }),
            ],
            SS::SourceFundedTargetRefunded(SourceFundedTargetRefunded {
                ref swap,
                ref source_htlc_id,
                ..
            })
            | SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref swap,
                ref source_htlc_id,
                ..
            }) => vec![Action::RefundHtlc(BitcoinRefund {
                outpoint: source_htlc_id.clone(),
                htlc: bitcoin_htlc(swap),
                value: swap.source_asset,
                transient_keypair: swap.source_identity.into(),
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_id,
                ref swap,
                ..
            })
            | SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_id,
                ref swap,
                ..
            }) => vec![Action::RedeemHtlc(EtherRedeem {
                contract_address: target_htlc_id.clone(),
                execution_gas: 42, //TODO: generate cas cost correctly
                data: swap.secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
