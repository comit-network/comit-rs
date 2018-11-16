use ethereum_support::Bytes;
use hex;

pub use self::{erc20_htlc::*, ether_htlc::*, queries::*};
use ethereum_support::{web3::types::Address, EtherQuantity};
use std::time::Duration;
use swap_protocols::{
    asset::Asset,
    ledger::Ethereum,
    rfc003::{state_machine::OngoingSwap, IntoSecretHash, Ledger},
};

mod erc20_htlc;
mod ether_htlc;
mod extract_secret;
mod queries;

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

pub fn ethereum_htlc<SL: Ledger, SA: Asset, S: IntoSecretHash>(
    swap: &OngoingSwap<SL, Ethereum, SA, EtherQuantity, S>,
) -> Box<Htlc> {
    Box::new(EtherHtlc::new(
        swap.target_ledger_lock_duration,
        swap.target_ledger_refund_identity,
        swap.target_ledger_success_identity,
        swap.secret.clone().into(),
    ))
}
