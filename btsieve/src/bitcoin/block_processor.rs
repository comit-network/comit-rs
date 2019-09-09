use crate::{bitcoin::queries::TransactionQuery, ArcQueryRepository, QueryMatch};
use bitcoin_support::Block;
use itertools::Itertools;

pub fn check_transaction_queries(
    transaction_queries: ArcQueryRepository<TransactionQuery>,
    block: Block,
) -> impl Iterator<Item = QueryMatch> {
    block
        .txdata
        .as_slice()
        .iter()
        .map(|transaction| {
            log::trace!("Processing {:?}", transaction);

            let transaction = transaction.clone();
            let transaction_id = transaction.txid().to_string();

            transaction_queries
                .all()
                .filter_map(move |(query_id, query)| {
                    log::trace!(
                        "Matching query {:#?} against transaction {:#?}",
                        query,
                        &transaction
                    );

                    if query.matches(&transaction) {
                        let transaction_id = transaction_id.clone();

                        log::trace!(
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
