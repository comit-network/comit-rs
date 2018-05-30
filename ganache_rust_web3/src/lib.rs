extern crate web3;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use web3::Transport;
use web3::api::Namespace;
use web3::helpers::CallResult;
use web3::types::{BlockId, BlockNumber, U256};

#[derive(Debug, Clone)]
pub struct Ganache<T: Transport> {
    transport: T,
}

#[derive(Serialize, Deserialize)]
pub struct SnapshotId(U256);

impl<T: Transport> Ganache<T> {
    /// Snapshot the state of the blockchain at the current block. Takes no parameters. Returns the integer id of the snapshot created.
    pub fn evm_snapshot(&self) -> CallResult<SnapshotId, T::Out> {
        CallResult::new(self.transport.execute("evm_snapshot", vec![]))
    }

    /// Revert the state of the blockchain to a previous snapshot. Takes a single parameter, which is the snapshot id to revert to. If no snapshot id is passed it will revert to the latest snapshot. Returns true.
    pub fn evm_revert(&self, id: &SnapshotId) -> CallResult<bool, T::Out> {
        let id = web3::helpers::serialize(id);

        CallResult::new(self.transport.execute("evm_revert", vec![id]))
    }

    // TODO: This returns some weird crap. Figure out how to hide from the caller.
    /// Jump forward in time. Takes one parameter, which is the amount of time to increase in seconds. Returns the total time adjustment, in seconds.
    pub fn evm_increase_time(&self, seconds: u64) -> CallResult<u64, T::Out> {
        CallResult::new(
            self.transport
                .execute("evm_increaseTime", vec![serde_json::Value::from(seconds)]),
        )
    }

    // TODO: This returns "0x0". Figure out how to swallow and hide from the caller.
    /// Force a block to be mined. Takes no parameters. Mines a block independent of whether or not mining is started or stopped.
    pub fn evm_mine(&self) -> CallResult<String, T::Out> {
        CallResult::new(self.transport.execute("evm_mine", vec![]))
    }
}

impl<T: Transport> Namespace<T> for Ganache<T> {
    fn new(transport: T) -> Self {
        Ganache { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::var;
    use web3;
    use web3::futures::Future;
    use web3::transports;

    #[test]
    fn test_evm_snapshot() {
        let endpoint = var("GANACHE_ENDPOINT").unwrap();

        let (_eloop, transport) = transports::Http::new(&endpoint).unwrap();
        let web3 = web3::Web3::new(transport);

        let _ = web3.api::<Ganache<transports::Http>>()
            .evm_snapshot()
            .wait()
            .unwrap();
    }

    #[test]
    fn test_evm_revert() {
        let endpoint = var("GANACHE_ENDPOINT").unwrap();

        let (_eloop, transport) = transports::Http::new(&endpoint).unwrap();
        let web3 = web3::Web3::new(transport);

        let snapshot_id = web3.api::<Ganache<transports::Http>>()
            .evm_snapshot()
            .wait()
            .unwrap();

        let _ = web3.api::<Ganache<transports::Http>>()
            .evm_revert(&snapshot_id)
            .wait()
            .unwrap();
    }

    #[test]
    fn test_evm_increase_time() {
        let endpoint = var("GANACHE_ENDPOINT").unwrap();

        let (_eloop, transport) = transports::Http::new(&endpoint).unwrap();
        let web3 = web3::Web3::new(transport);

        //        let increase = U256::from(1);
        //
        let _ = web3.api::<Ganache<transports::Http>>()
            .evm_increase_time("0x0".to_string())
            .wait()
            .unwrap();

        let _ = web3.api::<Ganache<transports::Http>>()
            .evm_mine()
            .wait()
            .unwrap();
    }

    #[test]
    fn test_evm_mine() {
        let endpoint = var("GANACHE_ENDPOINT").unwrap();

        let (_eloop, transport) = transports::Http::new(&endpoint).unwrap();
        let web3 = web3::Web3::new(transport);

        let result = web3.api::<Ganache<transports::Http>>()
            .evm_mine()
            .wait()
            .unwrap();

        println!("{:?}", result);
    }
}
