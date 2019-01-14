use crate::{
    ethereum::queries::{
        EthereumBlockQuery, EthereumTransactionLogQuery, EthereumTransactionQuery,
    },
    query_repository::QueryRepository,
    web3::types::{Block, Transaction},
    QueryMatch, QueryMatchResult,
};
use std::sync::Arc;
use tokio;

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;
type BlockQueryResults = Vec<QueryMatch>;
type TransactionQueryResults = Vec<QueryMatch>;

pub fn process(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    transaction_log_queries: ArcQueryRepository<EthereumTransactionLogQuery>,
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    block: &Block<Transaction>,
) -> Result<(BlockQueryResults, TransactionQueryResults), ()> {
    let block_query_results = process_block_queries(block_queries, block);

    let transaction_queries = transaction_queries.clone();

    let transaction_query_results = block
        .transactions
        .iter()
        .map(move |transaction| {
            process_transaction_queries(transaction_queries.clone(), transaction)
        })
        .flatten()
        .collect();

    trace!("Processing {:?}", block);
    transaction_log_queries
        .iter()
        .filter_map(move |(query_id, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);

            if query.matches_block(&block) {
                trace!("Block {:?} matches Query-ID: {:?}", block_id, query_id);

                block.transactions.iter().map(|tx| {
                    if query.matches_transaction_receipt(transaction_receipt) {
                        trace!("Transaction {:?} matches Query-ID: {:?}", tx, query_id);

                        Some((query_id, tx.hash))
                    } else {
                        None
                    }
                })
            }
        })
        .collect();

    Ok((block_query_results, transaction_query_results))
}

fn process_block_queries(
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    block: &Block<Transaction>,
) -> BlockQueryResults {
    trace!("Processing {:?}", block);
    let block_id = format!("{:x}", block.hash.unwrap()); // TODO should probably not unwrap here

    block_queries
        .clone()
        .all()
        .filter_map(|(query_id, query)| {
            trace!("Matching query {:#?} against block {:#?}", query, block);

            match query.matches(block) {
                QueryMatchResult::Yes { .. } => {
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
