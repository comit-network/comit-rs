use crate::{
    ethereum::{Address as EthereumAddress, Bytes, EtherQuantity, Transaction},
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

impl FundAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type FundActionOutput = DeployContract;

    fn fund_action(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self::FundActionOutput {
        htlc_params.into()
    }
}
impl RefundAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type RefundActionOutput = CallContract;

    fn refund_action(
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: EthereumAddress,
        _secret_source: &dyn DeriveIdentities,
        _fund_transaction: &Transaction,
    ) -> Self::RefundActionOutput {
        let gas_limit = EtherHtlc::tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: None,
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: Some(htlc_params.expiry),
        }
    }
}
impl RedeemAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type RedeemActionOutput = CallContract;

    fn redeem_action(
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: EthereumAddress,
        _secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let data = Bytes::from(secret.as_raw_secret().to_vec());
        let gas_limit = EtherHtlc::tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: Some(data),
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: None,
        }
    }
}
