use crate::{
    asset,
    ethereum::{Bytes, Transaction},
    htlc_location, identity,
    swap_protocols::{
        actions::ethereum::{CallContract, DeployContract},
        ledger::Ethereum,
        rfc003::{
            actions::{MakeFundAction, MakeRedeemAction, MakeRefundAction},
            create_swap::HtlcParams,
            DeriveIdentities, Secret,
        },
    },
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;

impl MakeFundAction for (Ethereum, asset::Ether) {
    type HtlcParams = HtlcParams<Ethereum, asset::Ether, identity::Ethereum>;
    type Output = DeployContract;

    fn make_fund_action(htlc_params: Self::HtlcParams) -> Self::Output {
        let htlc = EtherHtlc::from(htlc_params.clone());
        let gas_limit = EtherHtlc::deploy_tx_gas_limit();

        DeployContract {
            data: htlc.into(),
            amount: htlc_params.asset.clone(),
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
        }
    }
}

impl MakeRefundAction for (Ethereum, asset::Ether) {
    type HtlcParams = HtlcParams<Ethereum, asset::Ether, identity::Ethereum>;
    type HtlcLocation = htlc_location::Ethereum;
    type FundTransaction = Transaction;
    type Output = CallContract;

    fn make_refund_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        _secret_source: &dyn DeriveIdentities,
        _fund_transaction: &Self::FundTransaction,
    ) -> Self::Output {
        let gas_limit = EtherHtlc::refund_tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: None,
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: Some(htlc_params.expiry),
        }
    }
}

impl MakeRedeemAction for (Ethereum, asset::Ether) {
    type HtlcParams = HtlcParams<Ethereum, asset::Ether, identity::Ethereum>;
    type HtlcLocation = htlc_location::Ethereum;
    type Output = CallContract;

    fn make_redeem_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        _secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::Output {
        let data = Bytes::from(secret.as_raw_secret().to_vec());
        let gas_limit = EtherHtlc::redeem_tx_gas_limit();

        CallContract {
            to: htlc_location,
            data: Some(data),
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
            min_block_timestamp: None,
        }
    }
}
