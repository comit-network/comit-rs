mod actions;
pub mod htlc_events;

pub use self::actions::*;
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc, Htlc};

use crate::swap_protocols::{
    ledger::Ethereum,
    rfc003::{state_machine::HtlcParams, Ledger},
};
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
            htlc_params.expiry,
            htlc_params.refund_identity,
            htlc_params.redeem_identity,
            htlc_params.secret_hash,
        )
    }
}

impl HtlcParams<Ethereum, EtherQuantity> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(self.clone()).compile_to_hex().into()
    }
}

impl From<HtlcParams<Ethereum, Erc20Token>> for Erc20Htlc {
    fn from(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> Self {
        Erc20Htlc::new(
            htlc_params.expiry,
            htlc_params.refund_identity,
            htlc_params.redeem_identity,
            htlc_params.secret_hash,
            htlc_params.asset.token_contract,
            htlc_params.asset.quantity.0,
        )
    }
}

impl HtlcParams<Ethereum, Erc20Token> {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).compile_to_hex().into()
    }
    pub fn funding_tx_payload(&self, htlc_location: Address) -> Bytes {
        Erc20Htlc::from(self.clone()).funding_tx_payload(htlc_location)
    }
}
