use crate::{
    actions::ethereum::{CallContract, DeployContract},
    ledger::Ethereum,
    rfc003::{
        actions::{FundAction, RedeemAction, RefundAction},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Secret,
    },
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use ethereum_support::{Address as EthereumAddress, Bytes, EtherQuantity, Transaction};

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
        _secret_source: &dyn SecretSource,
        _fund_transaction: &Transaction,
    ) -> Self::RefundActionOutput {
        let gas_limit = EtherHtlc::tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: None,
            gas_limit,
            network: htlc_params.ledger.network,
            min_block_timestamp: Some(htlc_params.expiry),
        }
    }
}
impl RedeemAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type RedeemActionOutput = CallContract;

    fn redeem_action(
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: EthereumAddress,
        _secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let data = Bytes::from(secret.as_raw_secret().to_vec());
        let gas_limit = EtherHtlc::tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: Some(data),
            gas_limit,
            network: htlc_params.ledger.network,
            min_block_timestamp: None,
        }
    }
}
