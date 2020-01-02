mod bitcoind_connector;
mod blockchain_info_connector;
mod transaction_ext;
mod transaction_pattern;

pub use self::{
    bitcoind_connector::BitcoindConnector, blockchain_info_connector::BlockchainInfoConnector,
    transaction_ext::TransactionExt, transaction_pattern::TransactionPattern,
};

use crate::btsieve::{BlockByHash, LatestBlock};
use bitcoin::{
    consensus::{encode::deserialize, Decodable},
    hashes::sha256d,
    BitcoinHash,
};
use futures::future::Future;
use futures_core::compat::Future01CompatExt;
use reqwest::{r#async::Client, Url};
use std::{collections::HashSet, fmt::Debug};

pub async fn matching_transaction<C, E>(
    mut blockchain_connector: C,
    pattern: TransactionPattern,
    reference_timestamp: Option<u32>,
) -> Result<bitcoin::Transaction, ()>
where
    C: LatestBlock<Block = bitcoin::Block, Error = E>
        + BlockByHash<Block = bitcoin::Block, BlockHash = sha256d::Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    // Verify that we can successfully connect to the blockchain connector and check
    // if the transaction is in the latest block.
    let latest_block = match blockchain_connector.latest_block().compat().await {
        Ok(block) => block,
        Err(e) => {
            log::error!("Failed to connect to the blockchain_connector: {:?}", e,);
            return Err(());
        }
    };
    if let Some(transaction) = check_block_against_pattern(&latest_block.clone(), &pattern) {
        return Ok(transaction.clone());
    };

    // We didn't find the transaction, now we need to do two things; keep polling
    // for latest block so that we see transactions in new blocks and also go
    // back up the blockchain until 'reference_timestamp' i.e., look back in the
    // past.

    let mut prev_blockhashes: HashSet<sha256d::Hash> = HashSet::new();
    let mut missing_block_futures: Vec<_> = Vec::new();

    let mut oldest_block: Option<bitcoin::Block> = Some(latest_block.clone());
    prev_blockhashes.insert(latest_block.bitcoin_hash());

    let prev_blockhash = latest_block.header.prev_blockhash;
    let future = blockchain_connector.block_by_hash(prev_blockhash).compat();
    missing_block_futures.push((future, prev_blockhash));

    loop {
        // Delay so that we don't overload the CPU in the event that
        // latest_block() and block_by_hash() resolve quickly.

        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;

        let mut new_missing_block_futures = Vec::new();
        for (block_future, blockhash) in missing_block_futures.into_iter() {
            match block_future.await {
                Ok(block) => {
                    match check_block_against_pattern(&block, &pattern) {
                        Some(transaction) => return Ok(transaction.clone()),
                        None => {
                            let prev_blockhash = block.header.prev_blockhash;
                            let unknown_parent = prev_blockhashes.insert(prev_blockhash);

                            if unknown_parent {
                                let future =
                                    blockchain_connector.block_by_hash(prev_blockhash).compat();
                                new_missing_block_futures.push((future, prev_blockhash));
                            }
                        }
                    };
                }
                Err(e) => {
                    log::warn!("Could not get block with hash {}: {:?}", blockhash, e);

                    let future = blockchain_connector.block_by_hash(blockhash).compat();
                    new_missing_block_futures.push((future, blockhash));
                }
            };
        }
        missing_block_futures = new_missing_block_futures;

        // Look back into the past (upto timestamp) for one block.

        if let (Some(block), Some(reference_timestamp)) =
            (oldest_block.as_ref(), reference_timestamp)
        {
            if block.header.time >= reference_timestamp {
                match blockchain_connector
                    .block_by_hash(block.header.prev_blockhash)
                    .compat()
                    .await
                {
                    Ok(block) => match check_block_against_pattern(&block, &pattern) {
                        Some(transaction) => return Ok(transaction.clone()),
                        None => {
                            oldest_block.replace(block);
                        }
                    },
                    Err(e) => log::warn!(
                        "Could not get block with hash {}: {:?}",
                        block.bitcoin_hash(),
                        e
                    ),
                };
            }
        }

        // Check if a new block has been mined.

        if let Ok(latest_block) = blockchain_connector.latest_block().compat().await {
            // If we can insert then we have not seen this block.
            if prev_blockhashes.insert(latest_block.bitcoin_hash()) {
                if let Some(transaction) = check_block_against_pattern(&latest_block, &pattern) {
                    return Ok(transaction.clone());
                };

                // In case we missed a block somehow, check this blocks parent.
                if !prev_blockhashes.contains(&latest_block.header.prev_blockhash) {
                    let prev_blockhash = latest_block.header.prev_blockhash;
                    let future = blockchain_connector.block_by_hash(prev_blockhash).compat();

                    missing_block_futures.push((future, prev_blockhash));
                }
            }
        }
    }
}

fn check_block_against_pattern<'b>(
    block: &'b bitcoin::Block,
    pattern: &TransactionPattern,
) -> Option<&'b bitcoin::Transaction> {
    block
        .txdata
        .iter()
        .find(|transaction| pattern.matches(transaction))
}

pub fn bitcoin_http_request_for_hex_encoded_object<T: Decodable>(
    request_url: Url,
    client: Client,
) -> impl Future<Item = T, Error = Error> {
    client
        .get(request_url)
        .send()
        .and_then(|mut response| response.text())
        .map_err(Error::Reqwest)
        .and_then(decode_response)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported network: {0}")]
    UnsupportedNetwork(String),
    #[error("reqwest: ")]
    Reqwest(#[from] reqwest::Error),
    #[error("hex: ")]
    Hex(#[from] hex::FromHexError),
    #[error("deserialization: ")]
    Deserialization(#[from] bitcoin::consensus::encode::Error),
}

pub fn decode_response<T: Decodable>(response_text: String) -> Result<T, Error> {
    let bytes = hex::decode(response_text.trim()).map_err(Error::Hex)?;
    deserialize(bytes.as_slice()).map_err(Error::Deserialization)
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn can_decode_tx_from_bitcoind_http_interface() {
        // the line break here is on purpose, as it is returned like that from bitcoind
        let transaction = r#"02000000014135047eff77c95bce4955f630bc3e334690d31517176dbc23e9345493c48ecf000000004847304402200da78118d6970bca6f152a6ca81fa8c4dde856680eb6564edb329ce1808207c402203b3b4890dd203cc4c9361bbbeb7ebce70110d4b07f411208b2540b10373755ba01feffffff02644024180100000017a9142464790f3a3fddb132691fac9fd02549cdc09ff48700a3e1110000000017a914c40a2c4fd9dcad5e1694a41ca46d337eb59369d78765000000
"#.to_owned();

        let bytes = decode_response::<bitcoin::Transaction>(transaction);

        assert_that(&bytes).is_ok();
    }

    #[test]
    fn can_decode_block_from_bitcoind_http_interface() {
        // the line break here is on purpose, as it is returned like that from bitcoind
        let block = r#"00000020837603de6069115e22e7fbf063c2a6e3bc3b3206f0b7e08d6ab6c168c2e50d4a9b48676dedc93d05f677778c1d83df28fd38d377548340052823616837666fb8be1b795dffff7f200000000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401650101ffffffff0200f2052a0100000023210205980e76eee77386241a3a7a5af65e910fb7be411b98e609f7c0d97c50ab8ebeac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000
"#.to_owned();

        let bytes = decode_response::<bitcoin::Block>(block);

        assert_that(&bytes).is_ok();
    }
}
