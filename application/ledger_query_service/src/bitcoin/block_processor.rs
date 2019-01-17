use crate::{
    bitcoin::queries::{BlockQuery, TransactionQuery},
    ArcQueryRepository, QueryMatch,
};
use bitcoin_support::{serialize::BitcoinHash, MinedBlock as Block, Transaction};
use futures::{future::join_all, Future};
use tokio;

pub fn check_block_queries(
    block_queries: ArcQueryRepository<BlockQuery>,
    block: Block,
) -> impl Future<Item = Vec<QueryMatch>, Error = ()> {
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
                Some(QueryMatch(query_id.into(), block_id))
            } else {
                None
            }
        });

        result_futures.push(result_future);
    }

    join_all(result_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
}

pub fn check_transaction_queries(
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    transaction: Transaction,
) -> impl Future<Item = Vec<QueryMatch>, Error = ()> + Send {
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
                Some(QueryMatch(query_id.into(), transaction_id))
            } else {
                None
            }
        });

        result_futures.push(result_future);
    }

    join_all(result_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
}
