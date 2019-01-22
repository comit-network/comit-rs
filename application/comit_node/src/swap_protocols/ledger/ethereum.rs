use crate::swap_protocols::ledger::{Ledger, Ledgers};
use ethereum_support::{Address, EtherQuantity, Network, Transaction, H256};
use secp256k1_support::PublicKey;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ethereum {
    pub network: Network,
}

impl Ethereum {
    pub fn new(network: Network) -> Self {
        Ethereum { network }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            network: Network::Regtest,
        }
    }
}

impl Ledger for Ethereum {
    type Quantity = EtherQuantity;
    type TxId = H256;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = Address;
    type Transaction = Transaction;

    fn address_for_identity(&self, address: Address) -> Address {
        address
    }
}

impl From<Ethereum> for Ledgers {
    fn from(ethereum: Ethereum) -> Self {
        Ledgers::Ethereum(ethereum)
    }
}
