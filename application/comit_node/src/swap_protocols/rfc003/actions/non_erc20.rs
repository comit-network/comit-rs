use crate::swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        bitcoin,
        ethereum::{self, EtherHtlc},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Ledger, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Address as EthereumAddress, Bytes, EtherQuantity};

pub trait CreateActions<L: Ledger, A: Asset> {
    type FundActionOutput;
    type RefundActionOutput;
    type RedeemActionOutput;

    fn fund_action(htlc_params: HtlcParams<L, A>) -> Self::FundActionOutput;

    fn refund_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput;

    fn redeem_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput;
}

impl CreateActions<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type FundActionOutput = bitcoin::SendToAddress;
    type RefundActionOutput = bitcoin::SpendOutput;
    type RedeemActionOutput = bitcoin::SpendOutput;

    fn fund_action(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self::FundActionOutput {
        let to = htlc_params.compute_address();

        bitcoin::SendToAddress {
            to,
            amount: htlc_params.asset,
            network: htlc_params.ledger.network,
        }
    }

    fn refund_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset,
                htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
            ),
            network: htlc_params.ledger.network,
        }
    }

    fn redeem_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset,
                htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
            ),
            network: htlc_params.ledger.network,
        }
    }
}

impl CreateActions<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type FundActionOutput = ethereum::ContractDeploy;
    type RefundActionOutput = ethereum::SendTransaction;
    type RedeemActionOutput = ethereum::SendTransaction;

    fn fund_action(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self::FundActionOutput {
        htlc_params.into()
    }

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
        }
    }

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
        }
    }
}
