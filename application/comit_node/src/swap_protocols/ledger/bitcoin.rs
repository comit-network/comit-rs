use bitcoin_support::{
    Address, IntoP2wpkhAddress, Network, PubkeyHash, Transaction, TransactionId,
};
use secp256k1_support::PublicKey;
use swap_protocols::ledger::Ledger;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Bitcoin {
    pub network: Network,
}

impl Bitcoin {
    pub fn new(network: Network) -> Self {
        Bitcoin { network }
    }

    pub fn regtest() -> Self {
        Bitcoin {
            network: Network::Regtest,
        }
    }
}

// TODO: fix with #376
impl Default for Bitcoin {
    fn default() -> Self {
        Bitcoin {
            network: Network::Regtest,
        }
    }
}

impl Ledger for Bitcoin {
    type TxId = TransactionId;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = PubkeyHash;
    type Transaction = Transaction;

    fn address_for_identity(&self, pubkeyhash: PubkeyHash) -> Address {
        pubkeyhash.into_p2wpkh_address(self.network)
    }
}
