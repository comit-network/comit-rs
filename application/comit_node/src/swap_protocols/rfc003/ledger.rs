use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols::{
    self,
    rfc003::secret::{ExtractSecret, Secret, SecretHash},
};

pub trait Ledger: LedgerExtractSecret {
    type LockDuration: PartialEq
        + Debug
        + Clone
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + 'static;
    type HtlcLocation: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
    type HtlcIdentity: Clone
        + Send
        + Sync
        + PartialEq
        + Debug
        + Into<<Self as swap_protocols::ledger::Ledger>::Identity>;
}

pub trait LedgerExtractSecret: swap_protocols::ledger::Ledger {
    fn extract_secret_from_transaction(
        txn: &Self::Transaction,
        secret_hash: &SecretHash,
    ) -> Option<Secret>;
}

impl<L: swap_protocols::ledger::Ledger> LedgerExtractSecret for L
where
    L::Transaction: ExtractSecret,
{
    fn extract_secret_from_transaction(
        txn: &Self::Transaction,
        secret_hash: &SecretHash,
    ) -> Option<Secret> {
        txn.extract_secret(secret_hash)
    }
}
