mod bitcoind_connector;
mod cache;
mod transaction_ext;
mod transaction_pattern;

pub use self::{
    bitcoind_connector::BitcoindConnector, cache::Cache, transaction_ext::TransactionExt,
    transaction_pattern::TransactionPattern,
};
use crate::btsieve::{BlockByHash, LatestBlock, Predates};
use crate::Never;
use bitcoin::{
    consensus::{encode::deserialize, Decodable},
    BitcoinHash, OutPoint,
};
use chrono::NaiveDateTime;
use futures_core::compat::Future01CompatExt;
use genawaiter::sync::Co;
use reqwest::{Client, Url};
use std::collections::HashSet;

type Hash = bitcoin::BlockHash;
type Block = bitcoin::Block;

// TODO: reevaluate naming of function
pub async fn watch_for_transaction<C>(
    blockchain_connector: C,
    start_of_swap: NaiveDateTime,
    from_outpoint: OutPoint,
    unlock_script: Vec<Vec<u8>>,
) -> anyhow::Result<bitcoin::Transaction>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + Clone,
{
    // TODO: Refactor later (get rid of TransactionPatterns)
    let pattern = TransactionPattern {
        to_address: None,
        from_outpoint: Some(from_outpoint),
        unlock_script: Some(unlock_script),
    };

    let transaction = matching_transaction(blockchain_connector, start_of_swap, |transaction| {
        pattern.matches(&transaction)
    })
    .await?;

    Ok(transaction)
}

// TODO: reevaluate naming of function
pub async fn watch_for_transaction_and_outpoint<C>(
    blockchain_connector: C,
    start_of_swap: NaiveDateTime,
    compute_address: bitcoin::Address,
) -> anyhow::Result<(bitcoin::Transaction, bitcoin::OutPoint)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + Clone,
{
    // TODO: Refactor later (get rid of TransactionPatterns)
    let pattern = TransactionPattern {
        to_address: Some(compute_address),
        from_outpoint: None,
        unlock_script: None,
    };

    let transaction = matching_transaction(blockchain_connector, start_of_swap, |transaction| {
        pattern.matches(&transaction)
    })
    .await?;

    // TODO: Change this to return transaction and OutPoint, move the ourpoint search in
    //    match receipt.contract_address {
    //        Some(location) => Ok((transaction, location)),
    //        None => Err(anyhow::anyhow!("contract address missing from receipt")),
    //    }

    // Outpoint code from htlc_events
    //        let transaction = matching_transaction(connector, pattern, start_of_swap)
    //            .await
    //            .context("failed to find transaction to deploy htlc")?;
    //
    //        let (vout, _txout) = transaction
    //            .find_output(&htlc_params.compute_address())
    //            .expect("Deployment transaction must contain outpoint described in pattern");

    unimplemented!()
}

pub async fn matching_transaction<C, F>(
    connector: C,
    start_of_swap: NaiveDateTime,
    matcher: F,
) -> anyhow::Result<bitcoin::Transaction>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + Clone,
    F: Fn(bitcoin::Transaction) -> bool,
{
    // TODO: Implement finding the transaction, can the OutPoint search be meshed in here directly?

    unimplemented!()
}

fn check_block_against_pattern<'b>(
    block: &'b Block,
    pattern: &TransactionPattern,
) -> Option<&'b bitcoin::Transaction> {
    block.txdata.iter().find(|transaction| {
        let result = pattern.matches(transaction);

        tracing::debug!(
            "matching {:?} against transaction {} yielded {}",
            pattern,
            transaction.txid(),
            result
        );

        result
    })
}

/// This function uses the `connector` to find blocks relevant to a swap.  To do
/// this we must get the latest block, for each latest block we receive we must
/// ensure that we saw its parent i.e., that we did not miss any blocks between
/// this latest block and the previous latest block we received.  Finally, we
/// must also get each block back until the time that the swap started i.e.,
/// look into the past (in case any action occurred on chain while we were not
/// watching).
///
/// It yields those blocks as part of the process.
async fn find_relevant_blocks<C>(
    mut connector: C,
    co: &Co<Block>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Never>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + Clone,
{
    let mut seen_blocks: HashSet<Hash> = HashSet::new();

    let block = connector.latest_block().compat().await?;
    co.yield_(block.clone()).await;

    let blockhash = block.bitcoin_hash();
    seen_blocks.insert(blockhash);

    // Look back in time until we get a block that predates start_of_swap.
    let parent_hash = block.header.prev_blockhash;
    walk_back_until(
        predates_start_of_swap(start_of_swap),
        connector.clone(),
        co,
        parent_hash,
    )
    .await?;

    loop {
        let block = connector.latest_block().compat().await?;
        co.yield_(block.clone()).await;

        let blockhash = block.bitcoin_hash();
        seen_blocks.insert(blockhash);

        // Look back along the blockchain for missing blocks.
        let parent_hash = block.header.prev_blockhash;
        if !seen_blocks.contains(&parent_hash) {
            walk_back_until(
                seen_block_or_predates_start_of_swap(seen_blocks.clone(), start_of_swap),
                connector.clone(),
                co,
                parent_hash,
            )
            .await?;
        }

        // The duration of this timeout could/should depend on the network
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    }
}

/// Walks the blockchain backwards from the given hash until the predicate given
/// in `stop_condition` returns `true`.
///
/// This functions yields all blocks as part of its process.
async fn walk_back_until<C, P>(
    stop_condition: P,
    connector: C,
    co: &Co<Block>,
    starting_blockhash: Hash,
) -> anyhow::Result<()>
where
    C: BlockByHash<Block = Block, BlockHash = Hash>,
    P: Fn(&Block) -> anyhow::Result<bool>,
{
    let mut blockhash = starting_blockhash;

    loop {
        let block = connector.block_by_hash(blockhash).compat().await?;

        co.yield_(block.clone()).await;

        if stop_condition(&block)? {
            return Ok(());
        } else {
            blockhash = block.header.prev_blockhash
        }
    }
}

/// Constructs a predicate that returns `true` if the given block predates the
/// start_of_swap timestamp.
fn predates_start_of_swap(start_of_swap: NaiveDateTime) -> impl Fn(&Block) -> anyhow::Result<bool> {
    move |block| Ok(block.predates(start_of_swap))
}

/// Constructs a predicate that returns `true` if we have seen the given block
/// or the block predates the start_of_swap timestamp.
fn seen_block_or_predates_start_of_swap(
    seen_blocks: HashSet<Hash>,
    start_of_swap: NaiveDateTime,
) -> impl Fn(&Block) -> anyhow::Result<bool> {
    move |block: &Block| {
        let have_seen_block = seen_blocks.contains(&block.bitcoin_hash());
        let predates_start_of_swap = predates_start_of_swap(start_of_swap)(block)?;

        Ok(have_seen_block || predates_start_of_swap)
    }
}

impl Predates for Block {
    fn predates(&self, timestamp: NaiveDateTime) -> bool {
        let unix_timestamp = timestamp.timestamp();
        let block_time = self.header.time as i64;

        block_time < unix_timestamp
    }
}

// TODO: ###############################// TODO: ###############################// TODO: ###############################
// TODO: Remove old code below

//pub async fn matching_transaction_old<C>(
//    mut blockchain_connector: C,
//    pattern: TransactionPattern,
//    start_of_swap: NaiveDateTime,
//) -> anyhow::Result<bitcoin::Transaction>
//where
//    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + Clone,
//{
//    // Verify that we can successfully addPeer to the blockchain connector and check
//    // if the transaction is in the latest block.
//    let latest_block = blockchain_connector.latest_block().compat().await?;
//    if let Some(transaction) = check_block_against_pattern(&latest_block.clone(), &pattern) {
//        return Ok(transaction.clone());
//    };
//
//    // We didn't find the transaction, now we need to do two things; keep polling
//    // for latest block so that we see transactions in new blocks and also go
//    // back up the blockchain until 'start_of_swap' i.e., look back in the
//    // past.
//
//    let mut prev_blockhashes: HashSet<Hash> = HashSet::new();
//    let mut missing_block_futures: Vec<_> = Vec::new();
//
//    let mut oldest_block: Option<Block> = Some(latest_block.clone());
//    prev_blockhashes.insert(latest_block.bitcoin_hash());
//
//    let prev_blockhash = latest_block.header.prev_blockhash;
//    let future = blockchain_connector.block_by_hash(prev_blockhash).compat();
//    missing_block_futures.push((future, prev_blockhash));
//
//    loop {
//        // Delay so that we don't overload the CPU in the event that
//        // latest_block() and block_by_hash() resolve quickly.
//
//        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
//
//        let mut new_missing_block_futures = Vec::new();
//        for (block_future, blockhash) in missing_block_futures.into_iter() {
//            match block_future.await {
//                Ok(block) => {
//                    match check_block_against_pattern(&block, &pattern) {
//                        Some(transaction) => return Ok(transaction.clone()),
//                        None => {
//                            let prev_blockhash = block.header.prev_blockhash;
//                            let unknown_parent = prev_blockhashes.insert(prev_blockhash);
//
//                            if unknown_parent {
//                                let future =
//                                    blockchain_connector.block_by_hash(prev_blockhash).compat();
//                                new_missing_block_futures.push((future, prev_blockhash));
//                            }
//                        }
//                    };
//                }
//                Err(e) => {
//                    tracing::warn!("Could not get block with hash {}: {}", blockhash, e);
//
//                    let future = blockchain_connector.block_by_hash(blockhash).compat();
//                    new_missing_block_futures.push((future, blockhash));
//                }
//            };
//        }
//        missing_block_futures = new_missing_block_futures;
//
//        // Look back into the past (upto timestamp) for one block.
//
//        if let Some(block) = oldest_block.as_ref() {
//            if !block.predates(start_of_swap) {
//                match blockchain_connector
//                    .block_by_hash(block.header.prev_blockhash)
//                    .compat()
//                    .await
//                {
//                    Ok(block) => match check_block_against_pattern(&block, &pattern) {
//                        Some(transaction) => return Ok(transaction.clone()),
//                        None => {
//                            oldest_block.replace(block);
//                        }
//                    },
//                    Err(e) => tracing::warn!(
//                        "Could not get block with hash {}: {}",
//                        block.bitcoin_hash(),
//                        e
//                    ),
//                };
//            }
//        }
//
//        // Check if a new block has been mined.
//
//        if let Ok(latest_block) = blockchain_connector.latest_block().compat().await {
//            // If we can insert then we have not seen this block.
//            if prev_blockhashes.insert(latest_block.bitcoin_hash()) {
//                if let Some(transaction) = check_block_against_pattern(&latest_block, &pattern) {
//                    return Ok(transaction.clone());
//                };
//
//                // In case we missed a block somehow, check this blocks parent.
//                if !prev_blockhashes.contains(&latest_block.header.prev_blockhash) {
//                    let prev_blockhash = latest_block.header.prev_blockhash;
//                    let future = blockchain_connector.block_by_hash(prev_blockhash).compat();
//
//                    missing_block_futures.push((future, prev_blockhash));
//                }
//            }
//        }
//    }
//}

pub async fn bitcoin_http_request_for_hex_encoded_object<T: Decodable>(
    request_url: Url,
    client: Client,
) -> anyhow::Result<T> {
    let response_text = client.get(request_url).send().await?.text().await?;
    let decoded_response = decode_response(response_text)?;

    Ok(decoded_response)
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

        let bytes = decode_response::<Block>(block);

        assert_that(&bytes).is_ok();
    }
}
