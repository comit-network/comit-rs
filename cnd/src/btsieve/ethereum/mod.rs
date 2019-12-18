mod transaction_pattern;
mod web3_connector;

pub use self::{
    transaction_pattern::{Event, Topic, TransactionPattern},
    web3_connector::Web3Connector,
};
use crate::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Block, Transaction, TransactionAndReceipt, TransactionReceipt, H256, U256},
};
use futures_core::{compat::Future01CompatExt, future::join, FutureExt, TryFutureExt};
use std::{collections::HashSet, fmt::Display, ops::Add};
use tokio::timer::Delay;

pub async fn matching_transaction<C, E>(
    blockchain_connector: C,
    pattern: TransactionPattern,
    reference_timestamp: Option<u32>,
) -> TransactionAndReceipt
where
    C: LatestBlock<Block = Option<Block<Transaction>>, Error = E>
        + BlockByHash<Block = Option<Block<Transaction>>, BlockHash = H256, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = H256, Error = E>
        + tokio::executor::Executor
        + Clone,
    E: Display + Send + 'static,
{
    let (block_queue, next_block) = async_std::sync::channel(1);
    let (find_parent_queue, next_find_parent) = async_std::sync::channel(5);
    let (look_in_the_past_queue, next_look_in_the_past) = async_std::sync::channel(5);

    let reference_timestamp = reference_timestamp.map(U256::from);

    spawn(blockchain_connector.clone(), {
        let mut connector = blockchain_connector.clone();
        let block_queue = block_queue.clone();
        let find_parent_queue = find_parent_queue.clone();
        let look_in_the_past_queue = look_in_the_past_queue.clone();

        async move {
            let mut sent_blockhashes: HashSet<H256> = HashSet::new();

            loop {
                Delay::new(std::time::Instant::now().add(std::time::Duration::from_secs(1)))
                    .compat()
                    .await
                    .unwrap();

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
                        log::warn!("Could not get latest block: {}", e);
                    }
                };
            }
        }
    });

    let (fetch_block_by_hash_queue, next_hash) = async_std::sync::channel(5);

    spawn(blockchain_connector.clone(), {
        let connector = blockchain_connector.clone();
        let block_queue = block_queue.clone();
        let fetch_block_by_hash_queue = fetch_block_by_hash_queue.clone();

        async move {
            loop {
                match next_hash.recv().await {
                    Some(blockhash) => {
                        match connector.block_by_hash(blockhash).compat().await {
                            Ok(Some(block)) => {
                                join(
                                    block_queue.send(block.clone()),
                                    find_parent_queue.send((blockhash, block.parent_hash)),
                                )
                                .await;
                            }
                            Ok(None) => {
                                log::warn!("Block with hash {} does not exist", blockhash);
                            }
                            Err(e) => {
                                log::warn!("Could not get block with hash {}: {}", blockhash, e);

                                fetch_block_by_hash_queue.send(blockhash).await
                            }
                        };
                    }
                    None => unreachable!("sender cannot be dropped"),
                }
            }
        }
    });

    spawn(blockchain_connector.clone(), {
        async move {
            let mut prev_blockhashes: HashSet<H256> = HashSet::new();

            loop {
                match next_find_parent.recv().await {
                    Some((blockhash, parent_blockhash)) => {
                        prev_blockhashes.insert(blockhash);

                        if !prev_blockhashes.contains(&parent_blockhash)
                            && prev_blockhashes.len() > 1
                        {
                            fetch_block_by_hash_queue.send(parent_blockhash).await
                        }
                    }
                    None => unreachable!("senders cannot be dropped"),
                }
            }
        }
    });

    spawn(blockchain_connector.clone(), {
        let connector = blockchain_connector.clone();

        async move {
            loop {
                match next_look_in_the_past.recv().await {
                    Some(parent_blockhash) => {
                        match connector.block_by_hash(parent_blockhash).compat().await {
                            Ok(Some(block)) => {
                                let younger_than_reference_timestamp = reference_timestamp
                                    .map(|reference_timestamp| {
                                        reference_timestamp <= block.timestamp
                                    })
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
                                    "Could not get block with hash {}: {}",
                                    parent_blockhash,
                                    e
                                );

                                look_in_the_past_queue.send(parent_blockhash).await
                            }
                        }
                    }
                    None => unreachable!("senders cannot be dropped"),
                }
            }
        }
    });

    let (matching_transaction_queue, matching_transaction) = async_std::sync::channel(1);

    spawn(blockchain_connector.clone(), {
        let connector = blockchain_connector.clone();

        async move {
            loop {
                match next_block.recv().await {
                    Some(block) => {
                        let needs_receipt = pattern.needs_receipts(&block);

                        for transaction in block.transactions.into_iter() {
                            if needs_receipt {
                                let result =
                                    connector.receipt_by_hash(transaction.hash).compat().await;

                                let receipt = match result {
                                    Ok(Some(receipt)) => receipt,
                                    Ok(None) => {
                                        log::warn!("Could not get transaction receipt");
                                        continue;
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "Could not retrieve transaction receipt for {}: {}",
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
                                let result =
                                    connector.receipt_by_hash(transaction.hash).compat().await;

                                let receipt = match result {
                                    Ok(Some(receipt)) => receipt,
                                    Ok(None) => {
                                        log::warn!("Could not get transaction receipt for matching transaction");
                                        continue;
                                    }
                                    Err(e) => {
                                        log::warn!(
                                                            "Could not retrieve transaction receipt for matching transaction {}: {}",
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
    });

    matching_transaction
        .recv()
        .await
        .expect("sender cannot be dropped")
}

fn spawn(
    mut executor: impl tokio::executor::Executor,
    future: impl std::future::Future<Output = ()> + Send + 'static + Sized,
) {
    executor
        .spawn(Box::new(future.unit_error().boxed().compat()))
        .unwrap()
}
