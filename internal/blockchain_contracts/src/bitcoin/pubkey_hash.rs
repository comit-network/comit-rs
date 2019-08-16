use bitcoin_hashes::hash160;
pub use bitcoin_hashes::sha256d::Hash as TransactionId;
use secp256k1::PublicKey;

// TODO: Contribute back to rust-bitcoin
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PubkeyHash(hash160::Hash);

impl From<hash160::Hash> for PubkeyHash {
    fn from(hash: hash160::Hash) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(
            <bitcoin_hashes::hash160::Hash as bitcoin_hashes::Hash>::hash(&public_key.serialize()),
        )
    }
}

impl From<PubkeyHash> for hash160::Hash {
    fn from(pubkey_hash: PubkeyHash) -> Self {
        pubkey_hash.0
    }
}
