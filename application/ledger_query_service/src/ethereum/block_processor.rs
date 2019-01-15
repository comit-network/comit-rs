use crate::{
    ethereum::queries::{
        EthereumBlockQuery, EthereumTransactionLogQuery, EthereumTransactionQuery,
    },
    query_repository::QueryRepository,
    web3::types::{Block, Transaction},
    QueryMatch, QueryMatchResult,
};
use ethereum_support::web3::{transports::Http, Web3};
use futures::{future::Future, stream};
use std::sync::Arc;
use tokio;

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;
type BlockQueryResults = Vec<QueryMatch>;
type TransactionQueryResults = Vec<QueryMatch>;

pub fn process(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    transaction_log_queries: ArcQueryRepository<EthereumTransactionLogQuery>,
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    client: Arc<Web3<Http>>,
    block: &Block<Transaction>,
) -> Result<(BlockQueryResults, TransactionQueryResults), ()> {
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

    Ok((block_query_results, transaction_query_results))
}

fn process_block_queries(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    block: &Block<Transaction>,
) -> BlockQueryResults {
    trace!("Processing {:?}", block);

    block_queries
        .clone()
        .all()
        .filter_map(|(query_id, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);

            match query.matches(block) {
                QueryMatchResult::Yes { .. } => {
                    let block_id = format!("{:x}", block.hash.unwrap()); // TODO should probably not unwrap here

                    trace!("Block {:?} matches Query-ID: {:?}", block_id, query_id);
                    Some((query_id, block_id.clone()))
                }
                _ => None,
            }
        })
        .collect()
}

// This function and the previous one are exactly the same, so we should
// reintroduce a trait.
fn process_transaction_queries(
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    transaction: &Transaction,
) -> TransactionQueryResults {
    trace!("Processing {:?}", transaction);

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
                    let transaction_id = format!("{:x}", transaction.hash);

                    trace!(
                        "Transaction {:?} matches Query-ID: {:?}",
                        transaction_id,
                        query_id
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
    let mut query_results = vec![];

    for (query_id, query) in transaction_log_queries.all() {
        trace!("Matching query {:#?} against block {:#?}", query, block);

        if query.matches_block(&block) {
            let block_id = format!("{:x}", block.hash.unwrap()); // TODO should probably not unwrap here
            trace!("Block {:?} matches Query-ID: {:?}", block_id, query_id);
            let futures = block
                .transactions
                .iter()
                .map(|transaction| client.eth().transaction_receipt(transaction.hash))
                .filter_map(|transaction_receipt| match transaction_receipt {
                    Some(transaction_receipt) => {
                        if query.matches_transaction_receipt(transaction_receipt) {
                            let transaction_id =
                                format!("{:x}", transaction_receipt.transaction_hash);

                            Some((query_id, transaction_id))
                        } else {
                            None
                        }
                    }
                    None => {
                        trace!(
                            "Could not retrieve transaction receipt for transaction {}",
                            transaction_receipt.transaction_hash
                        );
                        None
                    }
                })
                .collect();

            query_results.append(
                stream::futures_ordered(futures)
                    .filter_map(|item| item)
                    .collect()
                    .wait(),
            )
        }
    }
    query_results
}
