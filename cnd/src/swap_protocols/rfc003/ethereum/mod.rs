pub mod htlc_events;

use crate::{
    asset,
    ethereum::{Address, Bytes},
    swap_protocols::{
        actions::ethereum::DeployContract,
        ledger::Ethereum,
        rfc003::{create_swap::HtlcParams, Ledger},
    },
};
use blockchain_contracts::ethereum::rfc003::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug)]
pub struct ByteCode(pub String);

impl Into<Bytes> for ByteCode {
    fn into(self) -> Bytes {
        Bytes(hex::decode(self.0).unwrap())
    }
}

pub trait Htlc {
    fn compile_to_hex(&self) -> ByteCode;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Seconds(pub u64);

impl From<Duration> for Seconds {
    fn from(duration: Duration) -> Self {
        Seconds(duration.as_secs())
    }
}

impl From<Seconds> for Duration {
    fn from(seconds: Seconds) -> Duration {
        Duration::from_secs(seconds.0)
    }
}

impl Ledger for Ethereum {
    type HtlcLocation = Address;
}

impl From<HtlcParams<Ethereum, asset::Ether>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Ether>) -> Self {
        EtherHtlc::new(
            htlc_params.expiry.into(),
            htlc_params.refund_identity,
            htlc_params.redeem_identity,
            htlc_params.secret_hash.into(),
        )
    }
}

impl HtlcParams<Ethereum, asset::Ether> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(*self).into()
    }
}

impl From<HtlcParams<Ethereum, asset::Erc20>> for Erc20Htlc {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Erc20>) -> Self {
        Erc20Htlc::new(
            htlc_params.expiry.into(),
            htlc_params.refund_identity,
            htlc_params.redeem_identity,
            htlc_params.secret_hash.into(),
            htlc_params.asset.token_contract,
            htlc_params.asset.quantity.0,
        )
    }
}

impl HtlcParams<Ethereum, asset::Erc20> {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(*self).into()
    }
}

impl From<HtlcParams<Ethereum, asset::Ether>> for DeployContract {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Ether>) -> Self {
        let htlc = EtherHtlc::from(htlc_params);
        let gas_limit = htlc.deployment_gas_limit();

        DeployContract {
            data: htlc.into(),
            amount: htlc_params.asset,
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
        }
    }
}

impl From<HtlcParams<Ethereum, asset::Erc20>> for DeployContract {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Erc20>) -> Self {
        let htlc = Erc20Htlc::from(htlc_params);
        let gas_limit = htlc.deployment_gas_limit();

        DeployContract {
            data: htlc.into(),
            amount: asset::Ether::zero(),
            gas_limit,
            chain_id: htlc_params.ledger.chain_id,
        }
    }
}
