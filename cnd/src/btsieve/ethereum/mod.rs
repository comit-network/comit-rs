mod transaction_pattern;
mod web3_connector;

pub use self::{
    transaction_pattern::{Event, Topic, TransactionPattern},
    web3_connector::Web3Connector,
};
use crate::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Transaction, TransactionAndReceipt, TransactionReceipt, H256, U256},
};
use async_std::sync::{Receiver, Sender};
use futures_core::{compat::Future01CompatExt, future::join};
use std::{collections::HashSet, fmt::Debug};

type Hash = H256;
type Block = crate::ethereum::Block<Transaction>;

pub async fn matching_transaction<C, E>(
    connector: C,
    pattern: TransactionPattern,
    reference_timestamp: Option<u32>,
) -> TransactionAndReceipt
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    let (block_queue, next_block) = async_std::sync::channel(1);
    let (find_parent_queue, next_find_parent) = async_std::sync::channel(5);
    let (look_in_the_past_queue, next_look_in_the_past) = async_std::sync::channel(5);

    tokio::task::spawn(
        process_latest_blocks(
            connector.clone(),
            block_queue.clone(),
            find_parent_queue.clone(),
            look_in_the_past_queue.clone(),
        ),
    );

    let (fetch_block_by_hash_queue, next_hash) = async_std::sync::channel(5);

    tokio::task::spawn(
        process_blocks_by_hash(
            connector.clone(),
            block_queue.clone(),
            find_parent_queue.clone(),
            (fetch_block_by_hash_queue.clone(), next_hash),
        ),
    );

    tokio::task::spawn(
        process_next_find_parent(
            connector.clone(),
            next_find_parent.clone(),
            fetch_block_by_hash_queue.clone(),
        ),
    );

    tokio::task::spawn(
        process_next_look_in_the_past(
            connector.clone(),
            block_queue.clone(),
            (look_in_the_past_queue.clone(), next_look_in_the_past),
            reference_timestamp,
        ),
    );

    let (matching_transaction_queue, matching_transaction) = async_std::sync::channel(1);

    tokio::task::spawn(
        process_next_block(
            connector.clone(),
            next_block,
            matching_transaction_queue,
            pattern,
        ),
    );

    matching_transaction
        .recv()
        .await
        .expect("sender cannot be dropped")
}

/// Repeatedly fetches the latest block from the Ethereum blockchain connector.
/// For the first (and first only) block; sends the block to the
/// `look_in_the_past_queue` channel. On fetch of each unique block; sends block
/// to the `block_queue` and `find_parent_queue` channels.
async fn process_latest_blocks<C, E>(
    mut connector: C,
    block_queue: Sender<Block>,
    find_parent_queue: Sender<(Hash, Hash)>,
    look_in_the_past_queue: Sender<Hash>,
) where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    let mut sent_blockhashes: HashSet<H256> = HashSet::new();

    loop {
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;

        match connector.latest_block().compat().await {
            Ok(Some(block)) if block.hash.is_some() => {
                let blockhash = block.hash.expect("cannot fail");

                if !sent_blockhashes.contains(&blockhash) {
                    sent_blockhashes.insert(blockhash);

                    join(
                        block_queue.send(block.clone()),
                        find_parent_queue.send((blockhash, block.parent_hash)),
                    )
                    .await;

                    if sent_blockhashes.len() == 1 {
                        look_in_the_past_queue.send(block.parent_hash).await
                    };
                }
            }
            Ok(Some(_)) => {
                log::warn!("Ignoring block without blockhash");
            }
            Ok(None) => {
                log::warn!("Could not get latest block");
            }
            Err(e) => {
                log::warn!("Could not get latest block: {:?}", e);
            }
        };
    }
}

/// Processes block hashes from the `next_hash` receiver, fetches blocks from
/// the blockchain connector by block hash and enqueues the block on the
/// `block_queue`. Enqueues the block hash and parent block hash on the
/// `find_parent_queue`.
async fn process_blocks_by_hash<C, E>(
    connector: C,
    block_queue: Sender<Block>,
    find_parent_queue: Sender<(Hash, Hash)>,
    next_block_channel: (Sender<Hash>, Receiver<Hash>),
) where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    let (fetch_block_by_hash_queue, next_hash) = next_block_channel;
    let mut sent_blockhashes: HashSet<H256> = HashSet::new();

    loop {
        match next_hash.recv().await {
            Some(blockhash) => {
                match connector.block_by_hash(blockhash).compat().await {
                    Ok(Some(block)) => {
                        if !sent_blockhashes.contains(&blockhash) {
                            sent_blockhashes.insert(blockhash);

                            join(
                                block_queue.send(block.clone()),
                                find_parent_queue.send((blockhash, block.parent_hash)),
                            )
                            .await;
                        }
                    }
                    Ok(None) => {
                        log::warn!("Block with hash {} does not exist", blockhash);
                    }
                    Err(e) => {
                        log::warn!("Could not get block with hash {}: {:?}", blockhash, e);

                        fetch_block_by_hash_queue.send(blockhash).await
                    }
                };
            }
            None => unreachable!("sender cannot be dropped"),
        }
    }
}

/// Processes `next_find_parent` queue by adding parent_blockhash to the
/// `fetch_block_by_hash` if it has not already been seen.
async fn process_next_find_parent<C, E>(
    _connector: C,
    next_find_parent: Receiver<(Hash, Hash)>,
    fetch_block_by_hash_queue: Sender<Hash>,
) where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    let mut prev_blockhashes: HashSet<H256> = HashSet::new();

    loop {
        match next_find_parent.recv().await {
            Some((blockhash, parent_blockhash)) => {
                prev_blockhashes.insert(blockhash);

                if !prev_blockhashes.contains(&parent_blockhash) && prev_blockhashes.len() > 1 {
                    fetch_block_by_hash_queue.send(parent_blockhash).await
                }
            }
            None => unreachable!("senders cannot be dropped"),
        }
    }
}

/// Process hashes from the `next_look_in_the_past` receiver. Gets the block
/// for this hash from the blockchain connector, if the block is _not_ yet
/// further back in time than `reference_timestamp` then enqueues block on the
/// `block_queue` and the block hash of parent to the `look_in_the_past_queue`.
async fn process_next_look_in_the_past<C, E>(
    connector: C,
    block_queue: Sender<Block>,
    look_in_the_past_channel: (Sender<Hash>, Receiver<Hash>),
    reference_timestamp: Option<u32>,
) where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    let reference_timestamp = reference_timestamp.map(U256::from);
    let (look_in_the_past_queue, next_look_in_the_past) = look_in_the_past_channel;

    loop {
        match next_look_in_the_past.recv().await {
            Some(parent_blockhash) => {
                match connector.block_by_hash(parent_blockhash).compat().await {
                    Ok(Some(block)) => {
                        let younger_than_reference_timestamp = reference_timestamp
                            .map(|reference_timestamp| reference_timestamp <= block.timestamp)
                            .unwrap_or(false);
                        if younger_than_reference_timestamp {
                            join(
                                block_queue.send(block.clone()),
                                look_in_the_past_queue.send(block.parent_hash),
                            )
                            .await;
                        }
                    }
                    Ok(None) => {
                        log::warn!("Block with hash {} does not exist", parent_blockhash);
                    }
                    Err(e) => {
                        log::warn!(
                            "Could not get block with hash {}: {:?}",
                            parent_blockhash,
                            e
                        );
                        // Delay here otherwise the error code path can go into a hot loop.
                        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
                        look_in_the_past_queue.send(parent_blockhash).await
                    }
                }
            }
            None => unreachable!("senders cannot be dropped"),
        }
    }
}

/// This is the actual processing of the blocks. Gets receipts if needed,
/// matches transactions in the block using `pattern` and enqueues 'transaction
/// and receipt' onto the `matching_transaction_queue` if a matching transaction
/// is found.
async fn process_next_block<C, E>(
    connector: C,
    next_block: Receiver<Block>,
    matching_transaction_queue: Sender<TransactionAndReceipt>,
    pattern: TransactionPattern,
) where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: Debug + Send + 'static,
{
    loop {
        match next_block.recv().await {
            Some(block) => {
                let needs_receipt = pattern.needs_receipts(&block);

                for transaction in block.transactions.into_iter() {
                    if needs_receipt {
                        let result = connector.receipt_by_hash(transaction.hash).compat().await;

                        let receipt = match result {
                            Ok(Some(receipt)) => receipt,
                            Ok(None) => {
                                log::warn!("Could not get transaction receipt");
                                continue;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Could not retrieve transaction receipt for {}: {:?}",
                                    transaction.hash,
                                    e
                                );
                                continue;
                            }
                        };

                        if pattern.matches(&transaction, Some(&receipt)) {
                            matching_transaction_queue
                                .send(TransactionAndReceipt {
                                    transaction,
                                    receipt,
                                })
                                .await;
                        }
                    } else if pattern.matches(&transaction, None) {
                        let result = connector.receipt_by_hash(transaction.hash).compat().await;

                        let receipt = match result {
                            Ok(Some(receipt)) => receipt,
                            Ok(None) => {
                                log::warn!(
                                    "Could not get transaction receipt for matching transaction"
                                );
                                continue;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Could not retrieve transaction receipt for matching transaction {}: {:?}",
                                    transaction.hash,
                                    e
                                );
                                continue;
                            }
                        };

                        matching_transaction_queue
                            .send(TransactionAndReceipt {
                                transaction,
                                receipt,
                            })
                            .await;
                    }
                }
            }
            None => unreachable!("senders cannot be dropped"),
        }
    }
}
