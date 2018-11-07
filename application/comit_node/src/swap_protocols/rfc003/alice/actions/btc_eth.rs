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
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Fund(BitcoinFund {
                address: bitcoin_htlc_address(swap),
                value: swap.source_asset,
            })],
            SS::SourceFunded { .. } => vec![],
            SS::BothFunded(BothFunded {
                ref source_htlc_location,
                ref target_htlc_location,
                ref swap,
                ..
            }) => vec![
                Action::Redeem(EtherRedeem {
                    contract_address: *target_htlc_location,
                    execution_gas: 42, //TODO: generate gas cost directly
                    data: swap.secret,
                }),
                Action::Refund(BitcoinRefund {
                    outpoint: *source_htlc_location,
                    htlc: bitcoin_htlc(swap),
                    value: swap.source_asset,
                    transient_keypair: swap.source_identity,
                }),
            ],
            SS::SourceFundedTargetRefunded(SourceFundedTargetRefunded {
                ref swap,
                ref source_htlc_location,
                ..
            })
            | SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref swap,
                ref source_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                outpoint: *source_htlc_location,
                htlc: bitcoin_htlc(swap),
                value: swap.source_asset,
                transient_keypair: swap.source_identity,
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_location,
                ref swap,
                ..
            })
            | SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Redeem(EtherRedeem {
                contract_address: *target_htlc_location,
                execution_gas: 42, //TODO: generate cas cost correctly
                data: swap.secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
