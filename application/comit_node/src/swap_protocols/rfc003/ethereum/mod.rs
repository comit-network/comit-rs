use ethereum_support::Bytes;
use hex;
use swap_protocols::rfc003::state_machine::HtlcParams;

pub use self::{erc20_htlc::*, ether_htlc::*, queries::*};
use ethereum_support::{web3::types::Address, EtherQuantity};
use std::time::Duration;
use swap_protocols::{ledger::Ethereum, rfc003::Ledger};

mod erc20_htlc;
mod ether_htlc;
mod extract_secret;
mod queries;
mod validation;

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    type LockDuration = Seconds;
    type HtlcLocation = Address;
    type HtlcIdentity = Address;
}

impl From<HtlcParams<Ethereum, EtherQuantity>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self {
        EtherHtlc::new(
            htlc_params.lock_duration,
            htlc_params.refund_identity,
            htlc_params.success_identity,
            htlc_params.secret_hash,
        )
    }
}
