use crate::{
    ethereum::queries::{
        EthereumBlockQuery, EthereumTransactionLogQuery, EthereumTransactionQuery,
    },
    web3::types::{Block, Transaction},
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

    let transaction_log_query_results =
        process_transaction_log_queries(transaction_log_queries, client, block);

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
) -> TransactionQueryResults {
    trace!("Processing {:?}", block);

    let block_hash;
    match block.hash {
        None => return vec![],
        Some(hash) => block_hash = format!("{:x}", hash),
    }

    transaction_log_queries
        .all()
        .filter(|(_, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);
            query.matches_block(&block)
        })
        .map(|(query_id, query)| {
            trace!("Query {:?} matches block {:?}", query_id, block_hash);
            let futures: Vec<_> = block
                .transactions
                .iter()
                .map(|transaction| client.eth().transaction_receipt(transaction.hash))
                .collect();

            stream::futures_ordered(futures)
                .filter_map(|x| x)
                .filter(|transaction_receipt| {
                    query.matches_transaction_receipt(transaction_receipt.clone())
                })
                .map(|transaction_receipt| {
                    let transaction_id = format!("{:x}", transaction_receipt.transaction_hash);
                    trace!(
                        "Transaction {:?} matches Query-ID: {:?}",
                        transaction_id,
                        query_id
                    );

                    (query_id, transaction_id)
                })
                .collect()
                .wait()
        })
        .flatten()
        .flatten()
        .collect()
}
