use crate::{
    bitcoin::{self, bitcoin_http_request_for_hex_encoded_object},
    blocksource::BlockSource,
};
use bitcoin_support::Network;
use futures::Future;
use reqwest::r#async::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: String,
}

#[derive(Clone)]
pub struct BlockchainInfoHttpBlockSource {
    client: Client,
    network: Network,
}

impl BlockchainInfoHttpBlockSource {
    pub fn new(network: Network) -> Result<Self, bitcoin::Error> {
        // Currently configured for Mainnet only because blockchain.info does not
        // support hex-encoded block retrieval for testnet.

        if network != Network::Mainnet {
            log::error!(
                "Network {} not supported for bitcoin http blocksource",
                network
            );
            return Err(bitcoin::Error::UnsupportedNetwork(format!(
                "Network {} currently not supported for bitcoin http plocksource",
                network
            )));
        }

        Ok(Self {
            client: Client::new(),
            network,
        })
    }
}

impl BlockSource for BlockchainInfoHttpBlockSource {
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
        let latest_block_url = "https://blockchain.info/latestblock";
        let latest_block_without_tx = self
            .client
            .get(latest_block_url)
            .send()
            .map_err(bitcoin::Error::Reqwest)
            .and_then(move |mut response| {
                response
                    .json::<BlockchainInfoLatestBlock>()
                    .map_err(bitcoin::Error::Reqwest)
            });

        let cloned_self = self.clone();

        Box::new(
            latest_block_without_tx
                .and_then(move |latest_block| cloned_self.block_by_hash(latest_block.hash)),
        )
    }

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let raw_block_by_hash_url =
            format!("https://blockchain.info/rawblock/{}?format=hex", block_hash);

        let block = bitcoin_http_request_for_hex_encoded_object::<Self::Block>(
            raw_block_by_hash_url,
            self.client.clone(),
        );

        Box::new(block.inspect(|block| {
            log::trace!("Fetched block from blockchain.info: {:?}", block);
        }))
    }

    fn transaction_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Transaction, Error = Self::Error> + Send + 'static> {
        let raw_transaction_by_hash_url = format!(
            "https://blockchain.info/rawtx/{}?format=hex",
            transaction_hash
        );

        let transaction = bitcoin_http_request_for_hex_encoded_object::<Self::Transaction>(
            raw_transaction_by_hash_url,
            self.client.clone(),
        );

        Box::new(transaction.inspect(|transaction| {
            log::debug!(
                "Fetched transaction from blockchain.info: {:?}",
                transaction
            );
        }))
    }
}
