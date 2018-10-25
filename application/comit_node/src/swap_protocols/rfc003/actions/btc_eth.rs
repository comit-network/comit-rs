use bitcoin_support::BitcoinQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, actions::StateActions, bitcoin, messages::AcceptResponse, secret::Secret,
        state_machine::*, Ledger,
    },
};

use super::{Action, BitcoinFund, BitcoinRefund, EtherRedeem};

pub fn bitcoin_htlc<TA: Clone>(
    start: &Start<Bitcoin, Ethereum, BitcoinQuantity, TA>,
    response: &AcceptResponse<Bitcoin, Ethereum>,
) -> rfc003::bitcoin::Htlc {
    rfc003::bitcoin::Htlc::new(
        response.source_ledger_success_identity,
        start.source_identity,
        start.secret.hash(),
        start.source_ledger_lock_duration.into(),
    )
}

impl<TA: Clone> StateActions<BitcoinFund, EtherRedeem, BitcoinRefund>
    for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, TA>
{
    fn actions(&self) -> Vec<Action<BitcoinFund, EtherRedeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![],
            SS::Accepted(Accepted {
                ref start,
                ref response,
                ..
            }) => {
                let htlc = bitcoin_htlc(start, response);
                let address = htlc.compute_address(start.source_ledger.network());
                vec![Action::FundHtlc(BitcoinFund {
                    address,
                    value: start.source_asset,
                })]
            }
            SS::SourceFunded { .. } => vec![],
            SS::BothFunded(BothFunded {
                ref source_htlc_id,
                ref target_htlc_id,
                ref start,
                ref response,
                ..
            }) => vec![
                Action::RedeemHtlc(EtherRedeem {
                    contract_address: target_htlc_id.clone(),
                    execution_gas: unimplemented!(),
                    data: start.secret,
                }),
                Action::RefundHtlc(BitcoinRefund {
                    outpoint: source_htlc_id.clone(),
                    htlc: bitcoin_htlc(start, response),
                    value: start.source_asset,
                    transient_keypair: start.source_identity.into(),
                }),
            ],
            SS::SourceFundedTargetRefunded(SourceFundedTargetRefunded {
                ref start,
                ref response,
                ref source_htlc_id,
                ..
            })
            | SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref start,
                ref response,
                ref source_htlc_id,
                ..
            }) => vec![Action::RefundHtlc(BitcoinRefund {
                outpoint: source_htlc_id.clone(),
                htlc: bitcoin_htlc(start, response),
                value: start.source_asset,
                transient_keypair: start.source_identity.into(),
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_id,
                ref start,
                ..
            })
            | SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_id,
                ref start,
                ..
            }) => vec![Action::RedeemHtlc(EtherRedeem {
                contract_address: target_htlc_id.clone(),
                execution_gas: unimplemented!(),
                data: start.secret,
            })],
            SS::Error(ref e) => vec![],
            SS::Final(ref end_state) => vec![],
        }
    }
}
