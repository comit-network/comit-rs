use crate::{
    asset,
    ethereum::{Bytes, Transaction},
    identity,
    swap_protocols::{
        actions::ethereum::{CallContract, DeployContract},
        ledger::Ethereum,
        rfc003::{
            actions::{FundAction, RedeemAction, RefundAction},
            create_swap::HtlcParams,
            DeriveIdentities, Secret,
        },
    },
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;

impl FundAction<Ethereum, asset::Ether> for (Ethereum, asset::Ether) {
    type FundActionOutput = DeployContract;

    fn fund_action(
        htlc_params: HtlcParams<Ethereum, asset::Ether, crate::ethereum::Address>,
    ) -> Self::FundActionOutput {
        let htlc = EtherHtlc::from(htlc_params.clone());
        let gas_limit = EtherHtlc::deploy_tx_gas_limit();

        DeployContract {
            data: htlc.into(),
            amount: htlc_params.asset.clone(),
            gas_limit: gas_limit.into(),
            chain_id: htlc_params.ledger.chain_id,
        }
    }
}
impl RefundAction<Ethereum, asset::Ether> for (Ethereum, asset::Ether) {
    type RefundActionOutput = CallContract;

    fn refund_action(
        htlc_params: HtlcParams<Ethereum, asset::Ether, crate::ethereum::Address>,
        htlc_location: identity::Ethereum,
        _secret_source: &dyn DeriveIdentities,
        _fund_transaction: &Transaction,
    ) -> Self::RefundActionOutput {
        let gas_limit = EtherHtlc::refund_tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: None,
            gas_limit: gas_limit.into(),
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: Some(htlc_params.expiry),
        }
    }
}
impl RedeemAction<Ethereum, asset::Ether> for (Ethereum, asset::Ether) {
    type RedeemActionOutput = CallContract;

    fn redeem_action(
        htlc_params: HtlcParams<Ethereum, asset::Ether, crate::ethereum::Address>,
        htlc_location: identity::Ethereum,
        _secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let data = Bytes::from(secret.as_raw_secret().to_vec());
        let gas_limit = EtherHtlc::redeem_tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: Some(data),
            gas_limit: gas_limit.into(),
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: None,
        }
    }
}
