mod bitcoind;
mod fee;
mod wallet;

pub use ::bitcoin::{Address, Block, BlockHash, Network, Transaction, Txid};
pub use bitcoind::*;
pub use comit::asset::Bitcoin as Amount;
pub use fee::*;
pub use wallet::Wallet;

pub const SATS_IN_BITCOIN_EXP: u16 = 8;

// Looking at the mainnet on 2Oct2020, biggest segwit tx with 21 inputs
// Had a size of 2073vB. 3000 seems to be a safe bet.
pub const MAX_EXPECTED_TRANSACTION_VBYTE_WEIGHT: u64 = 3000;

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
