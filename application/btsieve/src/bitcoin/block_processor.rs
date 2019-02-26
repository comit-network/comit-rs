use crate::{
    bitcoin::queries::{BlockQuery, TransactionQuery},
    ArcQueryRepository, QueryMatch,
};
use bitcoin_support::{serialize::BitcoinHash, MinedBlock as Block};
use itertools::Itertools;

pub fn check_block_queries(
    block_queries: ArcQueryRepository<BlockQuery>,
    block: Block,
) -> impl Iterator<Item = QueryMatch> {
    trace!("Processing {:?}", block);

    let block_id = block.as_ref().bitcoin_hash().to_string();

    block_queries.all().filter_map(move |(query_id, query)| {
        trace!("Matching query {:#?} against block {:#?}", query, block);

        let block = block.clone();

        if query.matches(&block) {
            let block_id = block_id.clone();

            trace!("Query {:?} matches block {}", query_id, block_id);

            Some(QueryMatch(query_id.into(), block_id))
        } else {
            None
        }
    })
}

pub fn check_transaction_queries(
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    block: Block,
) -> impl Iterator<Item = QueryMatch> {
    block
        .as_ref()
        .txdata
        .as_slice()
        .iter()
        .map(|transaction| {
            trace!("Processing {:?}", transaction);

            let transaction = transaction.clone();
            let transaction_id = transaction.txid().to_string();

            transaction_queries
                .all()
                .filter_map(move |(query_id, query)| {
                    trace!(
                        "Matching query {:#?} against transaction {:#?}",
                        query,
                        &transaction
                    );

                    if query.matches(&transaction) {
                        let transaction_id = transaction_id.clone();

                        trace!(
                            "Query {:?} matches transaction: {}",
                            query_id,
                            transaction_id
                        );

                        Some(QueryMatch(query_id.into(), transaction_id))
                    } else {
                        None
                    }
                })
        })
        .kmerge()
}
