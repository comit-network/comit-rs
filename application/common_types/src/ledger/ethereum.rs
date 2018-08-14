use ethereum_support::{Address, EthereumQuantity};
use ledger::Ledger;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;
    type Time = SystemTime;
    type HtlcId = Address;

    fn symbol() -> String {
        String::from("ETH")
    }
}
