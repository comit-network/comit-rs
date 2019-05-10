use crate::swap_protocols::{
    ledger::Ethereum,
    rfc003::{ethereum, state_machine::HtlcParams, Secret, Timestamp},
};
use blockchain_contracts::ethereum::rfc003::erc20_htlc::Erc20Htlc;
use ethereum_support::{Bytes, Erc20Token, Network};

pub fn deploy_action(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> ethereum::ContractDeploy {
    htlc_params.into()
}

pub fn fund_action(
    htlc_params: HtlcParams<Ethereum, Erc20Token>,
    to_erc20_contract: ethereum_support::Address,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::CallContract {
    let network = htlc_params.ledger.network;
    let gas_limit = Erc20Htlc::fund_tx_gas_limit();

    ethereum::CallContract {
        to: to_erc20_contract,
        data: Erc20Htlc::transfer_erc20_tx_payload(
            htlc_params.asset.quantity.0,
            beta_htlc_location,
        ),
        gas_limit,
        network,
        min_block_timestamp: None,
    }
}

pub fn refund_action(
    network: Network,
    expiry: Timestamp,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::CallContract {
    let data = Bytes::default();
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::CallContract {
        to: beta_htlc_location,
        data,
        gas_limit,
        network,
        min_block_timestamp: Some(expiry),
    }
}

pub fn redeem_action(
    alpha_htlc_location: ethereum_support::Address,
    secret: Secret,
    network: Network,
) -> ethereum::CallContract {
    let data = Bytes::from(secret.raw_secret().to_vec());
    let gas_limit = Erc20Htlc::tx_gas_limit();

    ethereum::CallContract {
        to: alpha_htlc_location,
        data,
        gas_limit,
        network,
        min_block_timestamp: None,
    }
}
