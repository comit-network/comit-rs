use crate::{
    ethereum::{BlockQuery, EventQuery, TransactionQuery},
    web3::types::{Block, Transaction},
    ArcQueryRepository, QueryMatch,
};
use ethereum_support::web3::{transports::Http, Web3};
use futures::{
    future::Future,
    stream::{self, Stream},
};
use itertools::Itertools;
use std::sync::Arc;

pub fn check_block_queries(
    block_queries: ArcQueryRepository<BlockQuery>,
    block: Block<Transaction>,
) -> impl Iterator<Item = QueryMatch> {
    log::trace!("Processing {:?}", block);

    let block_id = block.hash.map(|block_hash| format!("{:x}", block_hash));

    block_queries
        .all()
        .filter_map(move |(query_id, query)| {
            block_id.clone().map(|block_id| (query_id, query, block_id))
        })
        .filter_map(move |(query_id, query, block_id)| {
            log::trace!("Matching query {:#?} against block {:#?}", query, block);

            if query.matches(&block) {
                log::trace!("Query {:?} matches block {:?}", query_id, block_id);
                Some(QueryMatch(query_id.into(), block_id.clone()))
            } else {
                None
            }
        })
}

pub fn check_transaction_queries(
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    block: Block<Transaction>,
) -> impl Iterator<Item = QueryMatch> {
    block
        .transactions
        .iter()
        .map(|transaction| {
            log::trace!("Processing {:?}", transaction);

            let transaction = transaction.clone();
            let transaction_id = format!("{:x}", transaction.hash);

            transaction_queries
                .all()
                .filter_map(move |(query_id, query)| {
                    log::trace!(
                        "Matching query {:#?} against transaction {:#?}",
                        query,
                        &transaction
                    );

                    if query.matches(&transaction) {
                        log::trace!(
                            "Query {:?} matches transaction {:?}",
                            query_id,
                            transaction_id
                        );
                        Some(QueryMatch(query_id.into(), transaction_id.clone()))
                    } else {
                        None
                    }
                })
        })
        .kmerge()
}

pub fn check_log_queries(
    log_queries: ArcQueryRepository<EventQuery>,
    client: Arc<Web3<Http>>,
    block: Block<Transaction>,
) -> impl Stream<Item = QueryMatch, Error = ()> {
    log::trace!("Processing {:?}", block);

    let block_id = block.hash.map(|block_id| format!("{:x}", block_id));

    let result_futures = log_queries
        .all()
        .filter(|(_, query)| {
            log::trace!("Matching query {:#?} against block {:#?}", query, block);
            query.matches_block(&block)
        })
        .map(|(query_id, query)| {
            log::trace!("Query {:?} matches block {:?}", query_id, block_id);

            let client = Arc::clone(&client);

            block.transactions.iter().map(move |transaction| {
                let query = query.clone();
                let transaction_id = transaction.hash;
                client
                    .eth()
                    .transaction_receipt(transaction_id)
                    .then(move |result| match result {
                        Ok(Some(ref receipt))
                            if query.matches_transaction_receipt(receipt.clone()) =>
                        {
                            let transaction_id = receipt.transaction_hash;
                            log::trace!(
                                "Transaction {:?} matches Query-ID: {:?}",
                                transaction_id,
                                query_id
                            );

                            Ok(Some(QueryMatch(
                                query_id.into(),
                                format!("{:x}", transaction_id),
                            )))
                        }
                        Err(e) => {
                            log::error!(
                                "Could not retrieve transaction receipt for {}: {}",
                                transaction_id,
                                e
                            );
                            Ok(None)
                        }
                        _ => Ok(None),
                    })
            })
        })
        .flatten();

    stream::futures_ordered(result_futures).filter_map(|x| x)
}
