use common_types::seconds::Seconds;
use ethereum_support::{Address, EthereumQuantity, H256};
use ledger::Ledger;
use secp256k1_support::PublicKey;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;
    type LockDuration = Seconds;
    type HtlcId = Address;
    type TxId = H256;
    type Pubkey = PublicKey;

    fn symbol() -> String {
        String::from("ETH")
    }
}
