use ethereum_support::{Address, EthereumQuantity, H256};
use ledger::Ledger;
use seconds::Seconds;

#[derive(Clone, Debug)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;
    type LockDuration = Seconds;
    type HtlcId = Address;
    type TxId = H256;

    fn symbol() -> String {
        String::from("ETH")
    }
}
