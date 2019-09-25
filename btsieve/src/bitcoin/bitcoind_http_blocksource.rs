use crate::{
    bitcoin::{self, bitcoin_http_request_for_hex_encoded_object},
    blocksource::BlockSource,
};
use bitcoin_support::Network;
use futures::Future;
use reqwest::r#async::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ChainInfo {
    bestblockhash: String,
}

#[derive(Clone)]
pub struct BitcoindHttpBlockSource {
    network: Network,
    base_url: String,
    client: Client,
}

impl BitcoindHttpBlockSource {
    pub fn new(url: String, network: Network) -> Self {
        Self {
            network,
            base_url: url,
            client: Client::new(),
        }
    }
}

impl BlockSource for BitcoindHttpBlockSource {
    type Error = bitcoin::Error;
    type Block = bitcoin_support::Block;
    type BlockHash = String;
    type TransactionHash = String;
    type Transaction = bitcoin_support::Transaction;
    type Network = bitcoin_support::Network;

    fn network(&self) -> Self::Network {
        self.clone().network
    }

    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let bitcoind_blockchain_info_url = format!("{}/rest/chaininfo.json", self.base_url);

        let latest_block_hash = self
            .client
            .get(bitcoind_blockchain_info_url.as_str())
            .send()
            .map_err(|e| {
                log::error!("Error when sending request to bitcoind");
                Self::Error::Reqwest(e)
            })
            .and_then(move |mut response| {
                response.json::<ChainInfo>().map_err(|e| {
                    log::error!("Error when deserialising the response from bitcoind");
                    Self::Error::Reqwest(e)
                })
            })
            .map(move |blockchain_info| blockchain_info.bestblockhash);

        let cloned_self = self.clone();

        Box::new(
            latest_block_hash
                .and_then(move |latest_block_hash| cloned_self.block_by_hash(latest_block_hash)),
        )
    }

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let raw_block_by_hash_url = format!("{}/rest/block/{}.hex", self.base_url, block_hash);

        let block = bitcoin_http_request_for_hex_encoded_object::<Self::Block>(
            raw_block_by_hash_url,
            self.client.clone(),
        );

        Box::new(block.inspect(|block| {
            log::trace!("Fetched block from bitcoind: {:?}", block);
        }))
    }

    fn transaction_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Transaction, Error = Self::Error> + Send + 'static> {
        let raw_transaction_by_hash_url =
            format!("{}/rest/tx/{}.hex", self.base_url, transaction_hash);

        let transaction = bitcoin_http_request_for_hex_encoded_object::<Self::Transaction>(
            raw_transaction_by_hash_url,
            self.client.clone(),
        );

        Box::new(transaction.inspect(|transaction| {
            log::debug!("Fetched transaction from bitcoind: {:?}", transaction);
        }))
    }
}
