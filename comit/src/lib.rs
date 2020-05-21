pub mod actions;
pub mod asset;
pub mod bitcoin;
pub mod btsieve;
pub mod ethereum;
pub mod halight;
pub mod hbit;
pub mod herc20;
pub mod htlc_location;
pub mod identity;
pub mod ledger;
pub mod lightning;
pub mod lnd;
pub mod network;
mod secret;
mod secret_hash;
mod swap_id;
mod timestamp;
pub mod transaction;

pub use self::{
    network::DialInformation,
    secret::Secret,
    secret_hash::SecretHash,
    swap_id::SharedSwapId,
    timestamp::{RelativeTime, Timestamp},
};
use digest::ToDigestInput;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Copy,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
#[strum(serialize_all = "lowercase")]
pub enum Protocol {
    Hbit,
    Halight,
    Herc20,
}

pub type Never = std::convert::Infallible;

impl ToDigestInput for Timestamp {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().to_bytes().to_vec()
    }
}

impl ToDigestInput for RelativeTime {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl ToDigestInput for ethereum::Address {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().as_bytes().to_vec()
    }
}

impl ToDigestInput for asset::Bitcoin {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToDigestInput for asset::Ether {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl ToDigestInput for asset::Erc20Quantity {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}
