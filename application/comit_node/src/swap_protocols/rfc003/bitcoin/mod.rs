use bitcoin_support::{Blocks, PubkeyHash, TransactionId};
use secp256k1_support::KeyPair;
use swap_protocols::{ledger::Bitcoin, rfc003::Ledger};

mod htlc;

pub use self::htlc::{Htlc, UnlockingError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HtlcIdentity(pub KeyPair);

impl From<HtlcIdentity> for PubkeyHash {
    fn from(htlc_identity: HtlcIdentity) -> Self {
        let (_, public_key) = htlc_identity.0.into();
        public_key.into()
    }
}

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcId = HtlcId;
    type HtlcIdentity = HtlcIdentity;
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct HtlcId {
    pub transaction_id: TransactionId,
    pub vout: u32,
}
