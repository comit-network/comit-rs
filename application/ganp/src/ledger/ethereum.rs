use common_types::seconds::Seconds;
use ethereum_support::{Address, EthereumQuantity, H256};
use ledger::Ledger;
use secp256k1_support::PublicKey;
use swap;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type Address = Address;
    type LockDuration = Seconds;
    type HtlcId = Address;
    type TxId = H256;
    type Pubkey = PublicKey;
    type Identity = Address;

    fn symbol() -> String {
        String::from("ETH")
    }

    fn address_for_identity(&self, address: Address) -> Address {
        address
    }
}

impl From<Ethereum> for swap::Ledger {
    fn from(_: Ethereum) -> Self {
        swap::Ledger::Ethereum
    }
}
