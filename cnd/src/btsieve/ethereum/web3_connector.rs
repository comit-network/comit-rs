use crate::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{BlockId, BlockNumber},
};
use anyhow::Context;
use futures::Future;
use futures_core::{FutureExt, TryFutureExt};
use reqwest::{Client, Url};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Web3Connector {
    web3: Arc<Client>,
    url: Url,
}

impl Web3Connector {
    pub fn new(node_url: Url) -> Self {
        Self {
            web3: Arc::new(Client::new()),
            url: node_url,
        }
    }
}

impl LatestBlock for Web3Connector {
    type Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>;
    type BlockHash = crate::ethereum::H256;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let web3 = self.web3.clone();
        let url = self.url.clone();

        let future = async move {
            let request = JsonRpcRequest::new("eth_getBlockByNumber", vec![
                serialize(BlockId::Number(BlockNumber::Latest))?,
                serialize(true)?,
            ]);

            let response = web3
                .post(url)
                .json(&request)
                .send()
                .await?
                .json::<JsonRpcResponse<crate::ethereum::Block<crate::ethereum::Transaction>>>()
                .await?;

            let block = match response {
                JsonRpcResponse::Success { result } => result,
                JsonRpcResponse::Error { code, message } => {
                    log::warn!(
                        "eth_getBlockByNumber request failed with {}: {}",
                        code,
                        message
                    );
                    return Ok(None);
                }
            };

            log::trace!(
                "Fetched block from web3: {:x}",
                block.hash.expect("blocks to have a hash")
            );

            Ok(Some(block))
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

#[derive(serde::Serialize)]
struct JsonRpcRequest<T> {
    id: String,
    jsonrpc: String,
    method: String,
    params: T,
}

impl<T> JsonRpcRequest<T> {
    fn new(method: &str, params: T) -> Self {
        Self {
            id: "1".to_owned(),
            jsonrpc: "2.0".to_owned(),
            method: method.to_owned(),
            params,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum JsonRpcResponse<T> {
    Success { result: T },
    Error { code: i64, message: String },
}

impl BlockByHash for Web3Connector {
    type Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>;
    type BlockHash = crate::ethereum::H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let web3 = self.web3.clone();
        let url = self.url.clone();

        let future = async move {
            let request = JsonRpcRequest::new("eth_getBlockByHash", vec![
                serialize(&block_hash)?,
                serialize(true)?,
            ]);

            let response = web3
                .post(url)
                .json(&request)
                .send()
                .await?
                .json::<JsonRpcResponse<crate::ethereum::Block<crate::ethereum::Transaction>>>()
                .await?;

            let block = match response {
                JsonRpcResponse::Success { result } => result,
                JsonRpcResponse::Error { code, message } => {
                    log::warn!(
                        "eth_getBlockByHash request failed with {}: {}",
                        code,
                        message
                    );
                    return Ok(None);
                }
            };

            log::trace!("Fetched block from web3: {:x}", block_hash);

            Ok(Some(block))
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

impl ReceiptByHash for Web3Connector {
    type Receipt = Option<crate::ethereum::TransactionReceipt>;
    type TransactionHash = crate::ethereum::H256;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = anyhow::Error> + Send + 'static> {
        let web3 = self.web3.clone();
        let url = self.url.clone();

        let future = async move {
            let request = JsonRpcRequest::new("eth_getTransactionReceipt", vec![serialize(
                transaction_hash,
            )?]);

            let response = web3
                .post(url)
                .json(&request)
                .send()
                .await?
                .json::<JsonRpcResponse<crate::ethereum::TransactionReceipt>>()
                .await?;

            let receipt = match response {
                JsonRpcResponse::Success { result } => result,
                JsonRpcResponse::Error { code, message } => {
                    log::warn!(
                        "eth_getTransactionReceipt request failed with {}: {}",
                        code,
                        message
                    );
                    return Ok(None);
                }
            };

            log::trace!("Fetched receipt from web3: {:x}", transaction_hash);

            Ok(Some(receipt))
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

fn serialize<T: Serialize>(t: T) -> anyhow::Result<serde_json::Value> {
    let value = serde_json::to_value(t).context("failed to serialize parameter")?;

    Ok(value)
}
