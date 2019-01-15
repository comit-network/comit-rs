use crate::{
    ethereum::queries::{
        EthereumBlockQuery, EthereumTransactionLogQuery, EthereumTransactionQuery,
    },
    web3::{
        self,
        types::{Block, Transaction},
    },
    ArcQueryRepository, QueryMatch, QueryMatchResult,
};
use ethereum_support::web3::{transports::Http, Web3};
use futures::{
    future::Future,
    stream::{self, Stream},
};
use std::sync::Arc;
use tokio;

type BlockQueryResults = Vec<QueryMatch>;
type TransactionQueryResults = Vec<QueryMatch>;
type TransactionLogQueryResults = Vec<QueryMatch>;

pub fn process(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    transaction_log_queries: ArcQueryRepository<EthereumTransactionLogQuery>,
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    client: Arc<Web3<Http>>,
    block: &Block<Transaction>,
) -> Result<
    (
        BlockQueryResults,
        TransactionQueryResults,
        TransactionLogQueryResults,
    ),
    (),
> {
    let block_query_results = process_block_queries(block_queries, block);

    let transaction_queries = transaction_queries.clone();
    let transaction_query_results: TransactionQueryResults = block
        .transactions
        .iter()
        .map(move |transaction| {
            process_transaction_queries(transaction_queries.clone(), transaction)
        })
        .flatten()
        .collect();
    let result = process_transaction_log_queries(transaction_log_queries, client, block);

    let transaction_log_query_results = match result {
        Ok(result) => result,
        Err(e) => {
            error!(
                "Could not execute transaction log queries on block {:?}: {:?}",
                block, e
            );
            vec![]
        }
    };

    Ok((
        block_query_results,
        transaction_query_results,
        transaction_log_query_results,
    ))
}

fn process_block_queries(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    block: &Block<Transaction>,
) -> BlockQueryResults {
    trace!("Processing {:?}", block);

    let block_hash;
    match block.hash {
        None => return vec![],
        Some(hash) => block_hash = format!("{:x}", hash),
    }

    block_queries
        .all()
        .filter_map(|(query_id, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);

            match query.matches(block) {
                QueryMatchResult::Yes { .. } => {
                    trace!("Query {:?} matches block {:?}", query_id, block_hash);
                    Some((query_id, block_hash.clone()))
                }
                _ => None,
            }
        })
        .collect()
}

fn process_transaction_queries(
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    transaction: &Transaction,
) -> TransactionQueryResults {
    trace!("Processing {:?}", transaction);

    let transaction_id = format!("{:x}", transaction.hash);

    transaction_queries
        .clone()
        .all()
        .filter_map(|(query_id, query)| {
            trace!(
                "Matching query {:#?} against transaction {:#?}",
                query,
                transaction
            );

            match query.matches(transaction) {
                QueryMatchResult::Yes { .. } => {
                    trace!(
                        "Query {:?} matches transaction {:?}",
                        query_id,
                        transaction_id
                    );
                    Some((query_id, transaction_id.clone()))
                }
                _ => None,
            }
        })
        .collect()
}

fn process_transaction_log_queries(
    transaction_log_queries: ArcQueryRepository<EthereumTransactionLogQuery>,
    client: Arc<Web3<Http>>,
    block: &Block<Transaction>,
) -> Result<TransactionQueryResults, web3::Error> {
    trace!("Processing {:?}", block);

    let block_hash;
    match block.hash {
        None => return Ok(vec![]),
        Some(hash) => block_hash = format!("{:x}", hash),
    }

    let futures = transaction_log_queries
        .all()
        .filter(|(_, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);
            query.matches_block(&block)
        })
        .map(|(query_id, query)| {
            trace!("Query {:?} matches block {:?}", query_id, block_hash);

            let client = client.clone();

            block.transactions.iter().map(move |transaction| {
                let query = query.clone();

                client
                    .eth()
                    .transaction_receipt(transaction.hash)
                    .and_then(move |receipt| match receipt {
                        Some(receipt) => {
                            if query.matches_transaction_receipt(receipt.clone()) {
                                let transaction_id = receipt.transaction_hash;
                                trace!(
                                    "Transaction {:?} matches Query-ID: {:?}",
                                    transaction_id,
                                    query_id
                                );

                                Ok(Some((query_id, format!("{:x}", transaction_id))))
                            } else {
                                Ok(None)
                            }
                        }
                        None => Ok(None),
                    })
            })
        })
        .flatten();

    stream::futures_ordered(futures)
        .filter_map(|x| x)
        .collect()
        .wait()
}
