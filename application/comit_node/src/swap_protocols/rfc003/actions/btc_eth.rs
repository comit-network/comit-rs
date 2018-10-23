use bitcoin_support::BitcoinQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, actions::StateActions, bitcoin, messages::AcceptResponse, secret::Secret,
        state_machine::*, Ledger,
    },
};

use super::{Action, BitcoinFund, BitcoinRefund, EtherRedeem};

pub fn bitcoin_htlc<TA>(
    request: &rfc003::messages::Request<Bitcoin, Ethereum, BitcoinQuantity, TA>,
    response: &AcceptResponse<Bitcoin, Ethereum>,
) -> rfc003::bitcoin::Htlc {
    rfc003::bitcoin::Htlc::new(
        response.source_ledger_success_identity,
        request.source_ledger_refund_identity,
        request.secret_hash.clone(),
        request.source_ledger_lock_duration.into(),
    )
}

impl<TA> StateActions<BitcoinFund, EtherRedeem, BitcoinRefund>
    for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, TA>
{
    fn actions(&self) -> Vec<Action<BitcoinFund, EtherRedeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Sent { .. } => vec![],
            SS::Accepted(Accepted {
                ref request,
                ref response,
            }) => {
                let htlc = bitcoin_htlc(request, response);
                let address = htlc.compute_address(request.source_ledger.network());
                vec![Action::FundHtlc(BitcoinFund {
                    address,
                    value: request.source_asset,
                })]
            }
            SS::SourceFunded { .. } => vec![],
            SS::BothFunded(BothFunded {
                ref source_htlc_id,
                ref target_htlc_id,
                ref request,
                ref response,
                ..
            }) => vec![
                Action::RedeemHtlc(EtherRedeem {
                    contract_address: target_htlc_id.clone(),
                    execution_gas: unimplemented!(),
                }),
                Action::RefundHtlc(BitcoinRefund {
                    htlc_id: source_htlc_id.clone(),
                    htlc: bitcoin_htlc(request, response),
                    value: request.source_asset,
                }),
            ],
            SS::SourceFundedTargetRefunded(SourceFundedTargetRefunded {
                ref request,
                ref response,
                ref source_htlc_id,
                ..
            })
            | SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref request,
                ref response,
                ref source_htlc_id,
                ..
            }) => vec![Action::RefundHtlc(BitcoinRefund {
                htlc_id: source_htlc_id.clone(),
                htlc: bitcoin_htlc(request, response),
                value: request.source_asset,
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_id,
                ..
            })
            | SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_id,
                ..
            }) => vec![Action::RedeemHtlc(EtherRedeem {
                contract_address: target_htlc_id.clone(),
                execution_gas: unimplemented!(),
            })],
            SS::Error(ref e) => vec![],
            SS::Final(ref end_state) => vec![],
        }
    }
}
