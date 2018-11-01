use super::{Action, BitcoinRedeem, EtherDeploy, EtherRefund, StateActions};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{self, messages::AcceptResponse, state_machine::*, SecretHash},
};

pub fn ethereum_htlc(
    start: &Start<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash>,
    response: &AcceptResponse<Bitcoin, Ethereum>,
) -> Box<rfc003::ethereum::Htlc> {
    Box::new(rfc003::ethereum::EtherHtlc::new(
        response.target_ledger_lock_duration.into(), //TODO where to get the right time lock from
        response.target_ledger_refund_identity,
        start.target_identity,
        start.secret.clone(),
    ))
}

pub fn bitcoin_htlc(
    start: &Start<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash>,
    response: &AcceptResponse<Bitcoin, Ethereum>,
) -> rfc003::bitcoin::Htlc {
    rfc003::bitcoin::Htlc::new(
        response.source_ledger_success_identity,
        start.source_identity,
        start.secret.clone(),
        start.source_ledger_lock_duration.into(),
    )
}

impl StateActions<EtherDeploy, BitcoinRedeem, EtherRefund>
    for SwapStates<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, SecretHash>
{
    fn actions(&self) -> Vec<Action<EtherDeploy, BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![], //TODO what action is needed on start?
            SS::Accepted { .. } => vec![],
            SS::SourceFunded(SourceFunded {
                ref start,
                ref response,
                ..
            }) => {
                let htlc = ethereum_htlc(start, response);
                vec![Action::FundHtlc(EtherDeploy {
                    data: htlc.compile_to_hex().into(),
                    value: start.target_asset,
                    gas_limit: 42, //TODO come up with correct gas_limit
                })]
            }
            SS::BothFunded(BothFunded {
                ref source_htlc_id,
                ref target_htlc_id,
                ref start,
                ref response,
                ..
            }) => vec![Action::RefundHtlc(EtherRefund {
                contract_address: target_htlc_id.clone(),
                execution_gas: 42, //TODO: generate gas cost directly
            })],
            SS::SourceFundedTargetRefunded { .. } => vec![],
            SS::SourceFundedTargetRedeemed { .. } => vec![],
            SS::SourceRefundedTargetFunded { .. } => vec![],
            SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref start,
                ref response,
                ref target_htlc_id,
                ref source_htlc_id,
                ref secret,
            }) => vec![Action::RedeemHtlc(BitcoinRedeem {
                outpoint: source_htlc_id.clone(),
                htlc: bitcoin_htlc(start, response),
                value: start.source_asset,
                transient_keypair: start.source_identity.into(),
                secret: *secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
