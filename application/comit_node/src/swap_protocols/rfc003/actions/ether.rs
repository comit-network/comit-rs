use crate::swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        actions::{FundAction, RedeemAction, RefundAction},
        ethereum::{self, EtherHtlc},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Secret,
    },
};
use ethereum_support::{Address as EthereumAddress, Bytes, EtherQuantity};

impl FundAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type FundActionOutput = ethereum::ContractDeploy;

    fn fund_action(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self::FundActionOutput {
        htlc_params.into()
    }
}
impl RefundAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type RefundActionOutput = ethereum::SendTransaction;

    fn refund_action(
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: EthereumAddress,
        _secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput {
        let data = Bytes::default();
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: htlc_location,
            data,
            gas_limit,
            amount: EtherQuantity::zero(),
            network: htlc_params.ledger.network,
            min_block_timestamp: Some(htlc_params.expiry),
        }
    }
}
impl RedeemAction<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type RedeemActionOutput = ethereum::SendTransaction;

    fn redeem_action(
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: EthereumAddress,
        _secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let data = Bytes::from(secret.raw_secret().to_vec());
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: htlc_location,
            data,
            gas_limit,
            amount: EtherQuantity::zero(),
            network: htlc_params.ledger.network,
            min_block_timestamp: None,
        }
    }
}
