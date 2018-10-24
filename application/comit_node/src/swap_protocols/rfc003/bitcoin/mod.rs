use bitcoin_support::{Blocks, PubkeyHash, TransactionId};
use secp256k1_support::KeyPair;
use swap_protocols::{ledger::Bitcoin, rfc003::Ledger};

mod htlc;

pub use self::htlc::{Htlc, UnlockingError};

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcId = HtlcId;
    type HtlcIdentity = KeyPair;
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct HtlcId {
    pub transaction_id: TransactionId,
    pub vout: u32,
}
