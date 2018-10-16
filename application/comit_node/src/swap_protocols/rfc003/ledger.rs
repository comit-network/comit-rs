use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

pub trait Ledger: swap_protocols::ledger::Ledger {
    type LockDuration: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
}

pub mod bitcoin {

    use super::Ledger;
    use bitcoin_support::{Blocks, TransactionId};
    use swap_protocols::ledger::bitcoin::Bitcoin;

    impl Ledger for Bitcoin {
        type LockDuration = Blocks;
        type HtlcId = HtlcId;
    }

    #[derive(Clone, Deserialize, Serialize, Debug)]
    pub struct HtlcId {
        pub transaction_id: TransactionId,
        pub vout: u32,
    }

}

pub mod ethereum {

    use super::Ledger;
    use ethereum_support::web3::types::Address;
    use swap_protocols::{ledger::ethereum::Ethereum, rfc003::ethereum::Seconds};

    impl Ledger for Ethereum {
        type LockDuration = Seconds;
        type HtlcId = Address;
    }

}
