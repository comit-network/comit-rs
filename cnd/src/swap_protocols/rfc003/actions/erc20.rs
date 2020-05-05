use crate::{
    asset,
    ethereum::Bytes,
    htlc_location, identity,
    swap_protocols::{
        actions::ethereum::{CallContract, DeployContract},
        ledger::{ethereum::ChainId, Ethereum},
        rfc003::{create_swap::HtlcParams, Secret},
    },
    timestamp::Timestamp,
};
use blockchain_contracts::ethereum::rfc003::erc20_htlc::Erc20Htlc;

pub fn deploy_action(
    htlc_params: HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>,
) -> DeployContract {
    let chain_id = htlc_params.ledger.chain_id;
    let htlc = Erc20Htlc::from(htlc_params);
    let gas_limit = Erc20Htlc::deploy_tx_gas_limit();

    DeployContract {
        data: htlc.into(),
        amount: asset::Ether::zero(),
        gas_limit,
        chain_id,
    }
}

pub fn fund_action(
    htlc_params: HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>,
    to_erc20_contract: identity::Ethereum,
    beta_htlc_location: htlc_location::Ethereum,
) -> CallContract {
    let chain_id = htlc_params.ledger.chain_id;
    let gas_limit = Erc20Htlc::fund_tx_gas_limit();
    let beta_htlc_address = blockchain_contracts::ethereum::Address(beta_htlc_location.into());

    let data =
        Erc20Htlc::transfer_erc20_tx_payload(htlc_params.asset.quantity.into(), beta_htlc_address);

    CallContract {
        to: to_erc20_contract,
        data: Some(Bytes(data)),
        gas_limit,
        chain_id,
        min_block_timestamp: None,
    }
}

pub fn refund_action(
    chain_id: ChainId,
    expiry: Timestamp,
    beta_htlc_location: htlc_location::Ethereum,
) -> CallContract {
    let data = Bytes::default();
    let gas_limit = Erc20Htlc::refund_tx_gas_limit();

    CallContract {
        to: beta_htlc_location,
        data: Some(data),
        gas_limit,
        chain_id,
        min_block_timestamp: Some(expiry),
    }
}

pub fn redeem_action(
    alpha_htlc_location: htlc_location::Ethereum,
    secret: Secret,
    chain_id: ChainId,
) -> CallContract {
    let data = Bytes::from(secret.as_raw_secret().to_vec());
    let gas_limit = Erc20Htlc::redeem_tx_gas_limit();

    CallContract {
        to: alpha_htlc_location,
        data: Some(data),
        gas_limit,
        chain_id,
        min_block_timestamp: None,
    }
}
