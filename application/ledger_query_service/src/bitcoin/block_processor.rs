use crate::{
    bitcoin::queries::{BlockQuery, TransactionQuery},
    ArcQueryRepository, QueryMatch,
};
use bitcoin_support::{serialize::BitcoinHash, MinedBlock as Block, Transaction};
use futures::{future::join_all, Future};
use std::sync::Arc;
use tokio;

type BlockQueryResults = Vec<QueryMatch>;
type TransactionQueryResults = Vec<QueryMatch>;

pub fn process(
    block_queries: ArcQueryRepository<BlockQuery>,
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    block: Block,
) -> Box<dyn Future<Item = (BlockQueryResults, TransactionQueryResults), Error = ()> + Send> {
    let block_query_results = check_block_queries(block_queries, block.clone());

    let mut transaction_query_results = vec![];
    for transaction in block.as_ref().txdata.as_slice() {
        transaction_query_results.push(check_transaction_queries(
            Arc::clone(&transaction_queries),
            transaction.clone(),
        ))
    }
    let transaction_query_results = join_all(transaction_query_results)
        .map(|transaction_results_vec| transaction_results_vec.into_iter().flatten().collect());

    Box::new(block_query_results.join(transaction_query_results))
}

pub fn check_block_queries(
    block_queries: ArcQueryRepository<BlockQuery>,
    block: Block,
) -> impl Future<Item = BlockQueryResults, Error = ()> + Send {
    trace!("Processing {:?}", block);
    let mut result_futures = vec![];

    // We must collect the futures in a vector first to stop
    // borrow checker freaking out
    for (query_id, query) in block_queries.all() {
        trace!("Matching query {:#?} against block {:#?}", query, block);
        let block = block.clone();
        let result_future = query.matches(&block).map(move |block_matches| {
            if block_matches {
                let block_id = block.as_ref().bitcoin_hash().to_string();
                trace!("Query {:?} matches block {}", query_id, block_id);
                Some((query_id, block_id))
            } else {
                None
            }
        });

        result_futures.push(result_future);
    }

    join_all(result_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
}

fn check_transaction_queries(
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    transaction: Transaction,
) -> impl Future<Item = TransactionQueryResults, Error = ()> + Send {
    trace!("Processing {:?}", transaction);
    let mut result_futures = vec![];

    for (query_id, query) in transaction_queries.all() {
        trace!(
            "Matching query {:#?} against transaction {:#?}",
            query,
            transaction
        );
        let transaction = transaction.clone();
        let result_future = query.matches(&transaction).map(move |transaction_matches| {
            if transaction_matches {
                let transaction_id = transaction.txid().to_string();
                trace!(
                    "Query {:?} matches transaction: {}",
                    query_id,
                    transaction_id
                );
                Some((query_id, transaction_id))
            } else {
                None
            }
        });

        result_futures.push(result_future);
    }

    join_all(result_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
}
