use ethereum_support::{Address, EtherQuantity, H256};
use secp256k1_support::PublicKey;
use swap_protocols::{ledger::Ledger, rfc003::ethereum::Seconds};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EtherQuantity;
    type TxId = H256;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = Address;

    fn symbol() -> String {
        String::from("ETH")
    }

    fn address_for_identity(&self, address: Address) -> Address {
        address
    }
}
