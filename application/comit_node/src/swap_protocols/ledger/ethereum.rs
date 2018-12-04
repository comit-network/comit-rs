use ethereum_support::{Address, Transaction, H256};
use secp256k1_support::PublicKey;
use swap_protocols::ledger::Ledger;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type TxId = H256;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = Address;
    type Transaction = Transaction;

    fn address_for_identity(&self, address: Address) -> Address {
        address
    }
}
