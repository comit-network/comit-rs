pub mod htlc_events;

use crate::swap_protocols::{
    actions::ethereum::ContractDeploy,
    ledger::Ethereum,
    rfc003::{state_machine::HtlcParams, Ledger},
};
use blockchain_contracts::ethereum::rfc003::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};
use ethereum_support::{web3::types::Address, Bytes, Erc20Token, EtherQuantity};
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

impl From<HtlcParams<Ethereum, EtherQuantity>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self {
        EtherHtlc::new(
            htlc_params.expiry.into(),
            htlc_params.refund_identity,
            htlc_params.redeem_identity,
            htlc_params.secret_hash.into(),
        )
    }
}

impl HtlcParams<Ethereum, EtherQuantity> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(self.clone()).into()
    }
}

impl From<HtlcParams<Ethereum, Erc20Token>> for Erc20Htlc {
    fn from(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> Self {
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

impl HtlcParams<Ethereum, Erc20Token> {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).into()
    }
}

impl From<HtlcParams<Ethereum, EtherQuantity>> for ContractDeploy {
    fn from(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self {
        let htlc = EtherHtlc::from(htlc_params.clone());
        let gas_limit = htlc.deployment_gas_limit();

        ContractDeploy {
            data: htlc.into(),
            amount: htlc_params.asset,
            gas_limit,
            network: htlc_params.ledger.network,
        }
    }
}

impl From<HtlcParams<Ethereum, Erc20Token>> for ContractDeploy {
    fn from(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> Self {
        let htlc = Erc20Htlc::from(htlc_params.clone());
        let gas_limit = htlc.deployment_gas_limit();

        ContractDeploy {
            data: htlc.into(),
            amount: EtherQuantity::zero(),
            gas_limit,
            network: htlc_params.ledger.network,
        }
    }
}
