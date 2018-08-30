use ethereum_support::{Address, EthereumQuantity};
use ledger::Ledger;
use seconds::Seconds;

#[derive(Clone, Debug)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;
    type LockDuration = Seconds;
    type HtlcId = Address;

    fn symbol() -> String {
        String::from("ETH")
    }
}
