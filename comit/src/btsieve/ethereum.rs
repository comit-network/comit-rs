mod cache;
mod watch_for_contract_creation;
mod watch_for_event;
mod web3_connector;

pub use self::{
    cache::Cache,
    watch_for_contract_creation::{matching_transaction_and_receipt, watch_for_contract_creation},
    watch_for_event::watch_for_event,
    web3_connector::Web3Connector,
};
use crate::{
    btsieve::{BlockHash, ConnectedNetwork, Predates, PreviousBlockHash},
    ethereum::{Address, Block, ChainId, Hash, Log, Transaction, TransactionReceipt, U256},
};
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use time::OffsetDateTime;

#[async_trait]
pub trait ReceiptByHash: Send + Sync + 'static {
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> Result<TransactionReceipt>;
}

#[async_trait]
pub trait TransactionByHash: Send + Sync + 'static {
    async fn transaction_by_hash(&self, transaction_hash: Hash) -> Result<Transaction>;
}

#[async_trait]
pub trait GetLogs: Send + Sync + 'static {
    async fn get_logs(&self, event: Event) -> Result<Vec<Log>>;
}

impl BlockHash for Block {
    type BlockHash = Hash;

    fn block_hash(&self) -> Hash {
        self.hash
    }
}

impl PreviousBlockHash for Block {
    type BlockHash = Hash;

    fn previous_block_hash(&self) -> Hash {
        self.parent_hash
    }
}

impl Predates for Block {
    fn predates(&self, timestamp: OffsetDateTime) -> bool {
        let unix_timestamp = timestamp.timestamp();

        self.timestamp < U256::from(unix_timestamp)
    }
}

/// Event works similar to web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Event {
    pub address: Address,
    pub topics: Vec<Option<Hash>>,
}

async fn poll_interval<C>(connector: &C) -> Result<Duration>
where
    C: ConnectedNetwork<Network = ChainId>,
{
    let network = connector.connected_network().await?;
    let seconds = match network {
        ChainId::GETH_DEV => 1,
        _ => 10,
    };

    Ok(Duration::from_secs(seconds))
}
