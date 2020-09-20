mod bitcoind;
mod wallet;

pub use ::bitcoin::{Address, Network, Txid};
pub use bitcoind::*;
pub use comit::asset::Bitcoin as Amount;
pub use wallet::Wallet;

pub const SATS_IN_BITCOIN_EXP: u16 = 8;

#[cfg(test)]
pub mod amount {
    use super::*;

    pub fn btc(btc: f64) -> Amount {
        Amount::from_btc(btc).unwrap()
    }

    pub fn some_btc(btc: f64) -> Option<Amount> {
        Some(Amount::from_btc(btc).unwrap())
    }
}
