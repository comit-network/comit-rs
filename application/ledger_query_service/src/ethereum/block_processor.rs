use crate::{
    ethereum::queries::{EthereumBlockQuery, EthereumTransactionQuery},
    query_repository::QueryRepository,
    query_result_repository::QueryResultRepository,
    web3::types::{Block, Transaction},
    Query, QueryMatch, QueryMatchResult,
};
use futures::Future;
use std::sync::{Arc, Mutex};
use tokio;

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;
type ArcQueryResultRepository<Q> = Arc<dyn QueryResultRepository<Q>>;
type BlockQueryResults = Vec<QueryMatch>;
type TransactionQueryResults = Vec<QueryMatch>;

#[derive(DebugStub)]
pub struct BlockProcessor {
    #[debug_stub = "Queries"]
    transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
    #[debug_stub = "Queries"]
    block_queries: ArcQueryRepository<EthereumBlockQuery>,
    #[debug_stub = "Results"]
    transaction_results: ArcQueryResultRepository<EthereumTransactionQuery>,
}

impl BlockProcessor {
    pub fn new(
        transaction_query_repository: Arc<dyn QueryRepository<EthereumTransactionQuery>>,
        block_query_repository: Arc<dyn QueryRepository<EthereumBlockQuery>>,
        transaction_query_result_repository: Arc<
            dyn QueryResultRepository<EthereumTransactionQuery>,
        >,
    ) -> Self {
        Self {
            transaction_queries: transaction_query_repository,
            block_queries: block_query_repository,
            transaction_results: transaction_query_result_repository,
        }
    }
    pub fn process(
        block_queries: ArcQueryRepository<EthereumBlockQuery>,
        transaction_queries: ArcQueryRepository<EthereumTransactionQuery>,
        block: &Block<Transaction>,
    ) -> Result<(BlockQueryResults, TransactionQueryResults), ()> {
        let block_query_results = Self::process_block_queries(block_queries, block);

        let transaction_queries = transaction_queries.clone();

        let transaction_query_results = block
            .transactions
            .iter()
            .map(move |transaction| {
                Self::process_transaction_queries(transaction_queries.clone(), transaction)
            })
            .flatten()
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
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         in_memory_query_repository::InMemoryQueryRepository,
//         in_memory_query_result_repository::InMemoryQueryResultRepository,
// NonEmpty,     };
//     use spectral::prelude::*;

//     #[derive(Serialize, Deserialize, Clone, Default, Debug, Copy)]
//     struct GenericTransactionQuery {
//         transaction_id: u8,
//         confirmations_needed: u32,
//     }

//     impl Query<Transaction> for GenericTransactionQuery {
//         fn matches(
//             &self,
//             transaction: &GenericTransaction,
//         ) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send> {
//             if self.transaction_id == transaction.id {
//                 Box::new(futures::future::ok(
//
// QueryMatchResult::yes_with_confirmations(self.confirmations_needed),
//                 ))
//             } else {
//                 Box::new(futures::future::ok(QueryMatchResult::no()))
//             }
//         }
//     }

//     impl NonEmpty for GenericTransactionQuery {
//         fn is_empty(&self) -> bool {
//             false
//         }
//     }

//     #[derive(Serialize, Deserialize, Clone, Default, Debug, Copy)]
//     struct GenericBlockQuery {
//         min_timestamp_secs: u8,
//     }

//     impl Query<GenericBlock> for GenericBlockQuery {
//         fn matches(
//             &self,
//             block: &GenericBlock,
//         ) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send> {
//             if self.min_timestamp_secs <= block.timestamp {
//                 Box::new(futures::future::ok(QueryMatchResult::yes()))
//             } else {
//                 Box::new(futures::future::ok(QueryMatchResult::no()))
//             }
//         }
//     }

//     impl NonEmpty for GenericBlockQuery {
//         fn is_empty(&self) -> bool {
//             false
//         }
//     }

//     #[derive(Debug, Copy, Clone)]
//     struct GenericTransaction {
//         id: u8,
//     }

//     #[derive(Debug, Default, Clone)]
//     struct GenericBlock {
//         id: u8,
//         parent_id: u8,
//         timestamp: u8,
//         transaction_list: Vec<GenericTransaction>,
//     }

//     struct Setup {
//         block_processor: DefaultBlockProcessor,
//         first_block: GenericBlock,
//     }

//     impl Setup {
//         fn new(
//             query_transaction_id: u8,
//             transaction_id: u8,
//             confirmations_needed: u32,
//             query_timestamp: u8,
//             block_timestamp: u8,
//         ) -> Self {
//             let transaction_query_repository =
// Arc::new(InMemoryQueryRepository::default());             let
// transaction_query_result_repository =
// Arc::new(InMemoryQueryResultRepository::default());             let
// block_query_repository = Arc::new(InMemoryQueryRepository::default());

//             let block_processor = BlockProcessor::new(
//                 transaction_query_repository.clone(),
//                 block_query_repository.clone(),
//                 transaction_query_result_repository.clone(),
//             );

//             let first_transaction_query = GenericTransactionQuery {
//                 transaction_id: query_transaction_id,
//                 confirmations_needed,
//             };

//             transaction_query_repository
//                 .save(first_transaction_query)
//                 .unwrap();

//             let first_transaction = GenericTransaction { id: transaction_id
// };

//             let first_block_query = GenericBlockQuery {
//                 min_timestamp_secs: query_timestamp,
//             };

//             block_query_repository.save(first_block_query).unwrap();
//             let first_block = GenericBlock {
//                 id: 0,
//                 parent_id: 0,
//                 timestamp: block_timestamp,
//                 transaction_list: vec![first_transaction],
//             };

//             Self {
//                 block_processor,
//                 first_block,
//             }
//         }
//     }

//     #[test]
//     fn given_single_confirmation_query_when_matching_transaction_processes_returns_1_block_0_tx() {
//         let harness = Setup::new(1, 1, 1, 0, 0);
//         let mut block_processor = harness.block_processor;

//         let (blocks, transactions) =
// process_results(block_processor.process(harness.first_block));

//         assert_that(&blocks).named(&"found blocks").has_length(1);
//         assert_that(&transactions).named(&"found txs").has_length(1);
//     }

//     #[test]
//     #[ignore] // TODO fixme, pending transactions does not work correctly: https://github.com/comit-network/comit-rs/issues/591
//     fn given_double_confirmation_query_when_matching_transaction_is_processed_and_confirmed_adds_result(
//     ) {
//         let harness = Setup::new(1, 1, 2, 0, 0);
//         let mut block_processor = harness.block_processor;

//         let (blocks, transactions) =
// process_results(block_processor.process(harness.first_block));
//         assert_that(&blocks).named(&"found blocks").has_length(1);
//         // Transaction not yet confirmed
//         assert_that(&transactions).named(&"found txs").has_length(0);

//         let empty_block = GenericBlock::default();
//         let (blocks, transactions) =
// process_results(block_processor.process(empty_block));
//         assert_that(&blocks).named(&"found blocks").has_length(1);
//         assert_that(&transactions).named(&"found txs").has_length(1);
//     }

//     #[test]
//     fn given_single_confirmation_query_when_non_matching_transaction_process_returns_1_block_0_tx()
//     {
//         let harness = Setup::new(1, 2, 1, 0, 0);
//         let mut block_processor = harness.block_processor;

//         let (blocks, transactions) =
// process_results(block_processor.process(harness.first_block));
//         assert_that(&blocks).named(&"found blocks").has_length(1);
//         assert_that(&transactions).named(&"found txs").has_length(0);
//     }

//     #[test]
//     fn given_block_timestamp_query_when_younger_block_process_returns_1_block_0_tx() {
//         let harness = Setup::new(1, 2, 1, 5, 6);
//         let mut block_processor = harness.block_processor;

//         let (blocks, transactions) =
// process_results(block_processor.process(harness.first_block));
//         assert_that(&blocks).named(&"found blocks").has_length(1);
//         assert_that(&transactions).named(&"found txs").has_length(0);
//     }

//     #[test]
//     fn given_block_timestamp_query_when_older_block_process_returns_0_block_0_tx() {
//         let harness = Setup::new(1, 2, 1, 6, 5);
//         let mut block_processor = harness.block_processor;

//         let (blocks, transactions) =
// process_results(block_processor.process(harness.first_block));

//         assert_that(&blocks).named(&"found blocks").has_length(0);
//         assert_that(&transactions).named(&"found txs").has_length(0);
//     }

//     fn process_results(
//         processing_future: Box<
//             dyn Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()>
// + Send,         >,
//     ) -> (Vec<(u32, String)>, Vec<(u32, String)>) {
//         let mut runtime = tokio::runtime::Runtime::new().unwrap();
//         runtime.block_on(processing_future).unwrap()
//     }
// }
