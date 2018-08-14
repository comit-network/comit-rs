use ethereum_support::{Address, EthereumQuantity};
use ledger::Ledger;

#[derive(Clone, Debug)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;

    fn symbol() -> String {
        String::from("ETH")
    }
}
