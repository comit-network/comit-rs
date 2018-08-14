//TODO: create and use bitcoin_support blockheight
use bitcoin_rpc::BlockHeight;
use bitcoin_support::{Address, BitcoinQuantity};
use ledger::Ledger;

#[derive(Clone, Debug)]
pub struct Bitcoin {}

impl Ledger for Bitcoin {
    type Quantity = BitcoinQuantity;
    type Address = Address;
    type Time = BlockHeight;

    fn symbol() -> String {
        String::from("BTC")
    }
}
