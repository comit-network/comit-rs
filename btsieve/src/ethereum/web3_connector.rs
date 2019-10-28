use crate::{BlockByHash, LatestBlock, ReceiptByHash};
use ethereum_support::{
    web3::{
        self,
        futures::Future,
        transports::{EventLoopHandle, Http},
        types::BlockId,
        Web3,
    },
    BlockNumber,
};
use reqwest::Url;
use std::sync::Arc;

#[derive(Clone)]
pub struct Web3Connector {
    web3: Arc<Web3<Http>>,
}

impl Web3Connector {
    pub fn new(node_url: Url) -> Result<(Self, EventLoopHandle), web3::Error> {
        let (event_loop_handle, http_transport) = Http::new(node_url.as_str())?;
        Ok((
            Self {
                web3: Arc::new(Web3::new(http_transport)),
            },
            event_loop_handle,
        ))
    }
}

impl LatestBlock for Web3Connector {
    type Error = web3::Error;
    type Block = Option<ethereum_support::Block<ethereum_support::Transaction>>;
    type BlockHash = ethereum_support::H256;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(
            web.eth()
                .block_with_txs(BlockId::Number(BlockNumber::Latest)),
        )
    }
}

impl BlockByHash for Web3Connector {
    type Error = web3::Error;
    type Block = Option<ethereum_support::Block<ethereum_support::Transaction>>;
    type BlockHash = ethereum_support::H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().block_with_txs(BlockId::Hash(block_hash)))
    }
}

impl ReceiptByHash for Web3Connector {
    type Receipt = Option<ethereum_support::TransactionReceipt>;
    type TransactionHash = ethereum_support::H256;
    type Error = web3::Error;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().transaction_receipt(transaction_hash))
    }
}
