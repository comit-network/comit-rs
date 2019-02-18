use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use bitcoin_support::{
    Address, BitcoinQuantity, IntoP2wpkhAddress, Network, PubkeyHash, Transaction, TransactionId,
};
use secp256k1_support::PublicKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bitcoin {
    pub network: Network,
}

impl Bitcoin {
    pub fn new(network: Network) -> Self {
        Bitcoin { network }
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
    type Quantity = BitcoinQuantity;
    type TxId = TransactionId;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = PubkeyHash;
    type Transaction = Transaction;

    fn address_for_identity(&self, pubkeyhash: PubkeyHash) -> Address {
        pubkeyhash.into_p2wpkh_address(self.network)
    }
}

impl From<Bitcoin> for LedgerKind {
    fn from(bitcoin: Bitcoin) -> Self {
        LedgerKind::Bitcoin(bitcoin)
    }
}
