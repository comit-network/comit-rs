use crate::{
    query_repository::QueryRepository,
    query_result_repository::{QueryResult, QueryResultRepository},
};
use futures::{future::join_all, Future};
use std::{
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use tokio;

type QueryMatch = (u32, String);

pub trait BlockProcessor<B> {
    fn process(
        &mut self,
        block: B,
    ) -> Box<Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()> + Send>;
}

pub trait Transaction: Debug + 'static + Clone {
    fn transaction_id(&self) -> String;
}

pub trait Block: Debug + 'static + Clone {
    type Transaction: Transaction;

    fn blockhash(&self) -> String;
    fn prev_blockhash(&self) -> String;
    fn transactions(&self) -> &[Self::Transaction];
}

pub trait Query<O>: Debug + 'static {
    fn matches(&self, object: &O) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send>;
    fn is_empty(&self) -> bool;
}

#[derive(Debug, PartialEq)]
pub enum QueryMatchResult {
    Yes { confirmations_needed: u32 },
    No,
}

impl QueryMatchResult {
    pub fn yes() -> Self {
        QueryMatchResult::Yes {
            confirmations_needed: 0,
        }
    }
    pub fn yes_with_confirmations(confirmations_needed: u32) -> Self {
        QueryMatchResult::Yes {
            confirmations_needed,
        }
    }
    pub fn no() -> Self {
        QueryMatchResult::No
    }
}

#[derive(Debug)]
pub struct PendingTransaction {
    matching_query_id: u32,
    tx_id: String,
    pending_confirmations: u32,
}

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;
type ArcQueryResultRepository<Q> = Arc<dyn QueryResultRepository<Q>>;

#[derive(DebugStub)]
pub struct DefaultBlockProcessor<T, B, TQ, BQ> {
    #[debug_stub = "Queries"]
    transaction_queries: ArcQueryRepository<TQ>,
    #[debug_stub = "Queries"]
    block_queries: ArcQueryRepository<BQ>,
    #[debug_stub = "Results"]
    transaction_results: ArcQueryResultRepository<TQ>,
    #[debug_stub = "Results"]
    block_results: ArcQueryResultRepository<BQ>,
    pending_transactions: Arc<Mutex<Vec<PendingTransaction>>>,
    blockhashes: Vec<String>,
    tx_type: PhantomData<T>,
    block_type: PhantomData<B>,
}

impl<T: Transaction, B: Block<Transaction = T>, TQ: Query<T> + 'static, BQ: Query<B> + 'static>
    BlockProcessor<B> for DefaultBlockProcessor<T, B, TQ, BQ>
{
    fn process(
        &mut self,
        block: B,
    ) -> Box<Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()> + Send> {
        trace!("New block received: {:?}", block);
        // for now work on the assumption that there is one blockchain, but warn
        // every time that assumption doesn't hold, by comparing the previous
        // blockhash to the most recent member of a list of ordered blockhashes
        if let Some(last_blockhash) = self.blockhashes.last() {
            if *last_blockhash != block.prev_blockhash() {
                warn!(
                    "Block {} lists {} as previous block but last processed block was {}",
                    block.blockhash(),
                    block.prev_blockhash(),
                    last_blockhash
                );
            }
        }

        self.blockhashes.push(block.blockhash());
        self.update_pending_transactions();

        let block_futures = Self::process_new_block(self.block_queries.clone(), block.clone());

        let transaction_queries = self.transaction_queries.clone();
        let pending_transaction = self.pending_transactions.clone();
        let txns: Vec<T> = block.transactions().iter().map(Clone::clone).collect();

        let tx_results = join_all(txns.into_iter().map(move |tx| {
            Self::process_new_transaction(
                transaction_queries.clone(),
                pending_transaction.clone(),
                tx,
            )
        }))
        .map(|vec_vec_trnsaction_query_results| {
            vec_vec_trnsaction_query_results
                .into_iter()
                .flatten()
                .collect()
        });

        Box::new(block_futures.join(tx_results))
    }
}

impl<T: Transaction, B: Block<Transaction = T>, TQ: Query<T> + 'static, BQ: Query<B>>
    DefaultBlockProcessor<T, B, TQ, BQ>
{
    fn process_new_block(
        block_queries: ArcQueryRepository<BQ>,
        block: B,
    ) -> Box<Future<Item = Vec<QueryMatch>, Error = ()> + Send> {
        trace!("Processing {:?}", block);

        Box::new(
            join_all(block_queries.all().map(move |(query_id, query)| {
                trace!("Matching query {:#?} against block {:#?}", query, block);

                let block_id = block.blockhash();

                query
                    .matches(&block)
                    .map(move |query_match_result| match query_match_result {
                        QueryMatchResult::Yes { .. } => {
                            info!("Block {} matches Query-ID: {:?}", block_id, query_id);
                            Some((query_id, block_id))
                        }
                        QueryMatchResult::No => None,
                    })
            }))
            .map(|vec_query_result_options| {
                vec_query_result_options
                    .into_iter()
                    .filter_map(|x| x)
                    .collect()
            }),
        )
    }

    fn update_pending_transactions(&mut self) {
        trace!("Updating pending matching transactions");
        let mut pending_transactions = self.pending_transactions.lock().unwrap();
        pending_transactions
            .iter_mut()
            .for_each(|utx| utx.pending_confirmations -= 1);

        pending_transactions.iter().for_each(|utx| {
            if utx.pending_confirmations == 0 {
                let confirmed_tx_id = &utx.tx_id;
                trace!(
                    "Transaction {} now has enough confirmations. Sent to query result repository",
                    confirmed_tx_id
                );
                self.transaction_results
                    .add_result(utx.matching_query_id, confirmed_tx_id.clone())
            }
        });

        pending_transactions.retain(|ref utx| utx.pending_confirmations > 0);
    }

    fn process_new_transaction(
        transaction_queries: ArcQueryRepository<TQ>,
        pending_transactions: Arc<Mutex<Vec<PendingTransaction>>>,
        transaction: T,
    ) -> Box<Future<Item = Vec<QueryMatch>, Error = ()> + Send> {
        trace!("Processing {:?}", transaction);
        Box::new(
            join_all(transaction_queries.all().map(move |(query_id, query)| {
                trace!(
                    "Matching query {:#?} against transaction {:#?}",
                    query,
                    transaction
                );

                let tx_id = transaction.transaction_id();

                let pending_transactions = pending_transactions.clone();

                query.matches(&transaction).map(
                    move |query_match_result| match query_match_result {
                        QueryMatchResult::Yes {
                            confirmations_needed: 0,
                        }
                        | QueryMatchResult::Yes {
                            confirmations_needed: 1,
                        } => {
                            info!(
                                "Confirmed transaction {} matches Query-ID: {:?}",
                                tx_id, query_id
                            );
                            Some((query_id, tx_id))
                        }
                        QueryMatchResult::Yes {
                            confirmations_needed,
                        } => {
                            info!(
                                "Unconfirmed transaction {} matches Query-ID: {:?}",
                                tx_id, query_id
                            );
                            let pending_tx = PendingTransaction {
                                matching_query_id: query_id,
                                tx_id,
                                pending_confirmations: confirmations_needed - 1,
                            };
                            let mut pending_transactions = pending_transactions.lock().unwrap();
                            pending_transactions.push(pending_tx);
                            None
                        }
                        QueryMatchResult::No => None,
                    },
                )
            }))
            .map(|vec_query_result_options| {
                vec_query_result_options
                    .into_iter()
                    .filter_map(|x| x)
                    .collect()
            }),
        )
    }
}

impl<T, B, TQ, BQ> DefaultBlockProcessor<T, B, TQ, BQ> {
    pub fn new(
        transaction_query_repository: Arc<dyn QueryRepository<TQ>>,
        block_query_repository: Arc<dyn QueryRepository<BQ>>,
        transaction_query_result_repository: Arc<dyn QueryResultRepository<TQ>>,
        block_query_result_repository: Arc<dyn QueryResultRepository<BQ>>,
    ) -> Self {
        Self {
            transaction_queries: transaction_query_repository,
            block_queries: block_query_repository,
            transaction_results: transaction_query_result_repository,
            block_results: block_query_result_repository,
            pending_transactions: Arc::new(Mutex::new(Vec::new())),
            blockhashes: Vec::new(),
            tx_type: PhantomData,
            block_type: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        in_memory_query_repository::InMemoryQueryRepository,
        in_memory_query_result_repository::InMemoryQueryResultRepository,
    };

    #[derive(Serialize, Deserialize, Clone, Default, Debug, Copy)]
    struct GenericTransactionQuery {
        transaction_id: u8,
        confirmations_needed: u32,
    }

    impl Query<GenericTransaction> for GenericTransactionQuery {
        fn matches(
            &self,
            transaction: &GenericTransaction,
        ) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send> {
            if self.transaction_id == transaction.id {
                Box::new(futures::future::ok(
                    QueryMatchResult::yes_with_confirmations(self.confirmations_needed),
                ))
            } else {
                Box::new(futures::future::ok(QueryMatchResult::no()))
            }
        }
        fn is_empty(&self) -> bool {
            false
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug, Copy)]
    struct GenericBlockQuery {
        min_timestamp_secs: u8,
    }

    impl Query<GenericBlock> for GenericBlockQuery {
        fn matches(
            &self,
            block: &GenericBlock,
        ) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send> {
            if self.min_timestamp_secs <= block.timestamp {
                Box::new(futures::future::ok(QueryMatchResult::yes()))
            } else {
                Box::new(futures::future::ok(QueryMatchResult::no()))
            }
        }

        fn is_empty(&self) -> bool {
            false
        }
    }

    #[derive(Debug, Copy, Clone)]
    struct GenericTransaction {
        id: u8,
    }

    impl Transaction for GenericTransaction {
        fn transaction_id(&self) -> String {
            self.id.to_string()
        }
    }

    #[derive(Debug, Default, Clone)]
    struct GenericBlock {
        id: u8,
        parent_id: u8,
        timestamp: u8,
        transaction_list: Vec<GenericTransaction>,
    }

    impl Block for GenericBlock {
        type Transaction = GenericTransaction;

        fn blockhash(&self) -> String {
            self.id.to_string()
        }
        fn prev_blockhash(&self) -> String {
            self.parent_id.to_string()
        }
        fn transactions(&self) -> &[GenericTransaction] {
            self.transaction_list.as_slice()
        }
    }

    struct Setup {
        transaction_query_result_repository:
            Arc<InMemoryQueryResultRepository<GenericTransactionQuery>>,
        block_query_result_repository: Arc<InMemoryQueryResultRepository<GenericBlockQuery>>,
        block_processor: DefaultBlockProcessor<
            GenericTransaction,
            GenericBlock,
            GenericTransactionQuery,
            GenericBlockQuery,
        >,
        first_transaction_query_id: u32,
        first_block: GenericBlock,
        first_block_query_id: u32,
    }

    impl Setup {
        fn new(
            query_transaction_id: u8,
            transaction_id: u8,
            confirmations_needed: u32,
            query_timestamp: u8,
            block_timestamp: u8,
        ) -> Self {
            let transaction_query_repository = Arc::new(InMemoryQueryRepository::default());
            let transaction_query_result_repository =
                Arc::new(InMemoryQueryResultRepository::default());
            let block_query_repository = Arc::new(InMemoryQueryRepository::default());
            let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

            let block_processor = DefaultBlockProcessor::new(
                transaction_query_repository.clone(),
                block_query_repository.clone(),
                transaction_query_result_repository.clone(),
                block_query_result_repository.clone(),
            );

            let first_transaction_query = GenericTransactionQuery {
                transaction_id: query_transaction_id,
                confirmations_needed,
            };

            let first_query_id = transaction_query_repository
                .save(first_transaction_query)
                .unwrap();

            let first_transaction = GenericTransaction { id: transaction_id };

            let first_block_query = GenericBlockQuery {
                min_timestamp_secs: query_timestamp,
            };

            let first_block_query_id = block_query_repository.save(first_block_query).unwrap();

            let first_block = GenericBlock {
                id: 0,
                parent_id: 0,
                timestamp: block_timestamp,
                transaction_list: vec![first_transaction],
            };

            Self {
                transaction_query_result_repository,
                block_query_result_repository,
                block_processor,
                first_transaction_query_id: first_query_id,
                first_block,
                first_block_query_id,
            }
        }
    }

    #[test]
    fn given_single_confirmation_query_when_matching_transaction_is_processed_adds_result() {
        let harness = Setup::new(1, 1, 1, 0, 0);
        let mut block_processor = harness.block_processor;

        {
            let block_query_result_repository = harness.block_query_result_repository.clone();
            let transaction_query_result_repository =
                harness.transaction_query_result_repository.clone();

            block_processor
                .process(harness.first_block)
                .and_then(move |(block_results, transaction_results)| {
                    for (id, block_id) in block_results {
                        block_query_result_repository.add_result(id, block_id);
                    }
                    for (id, tx_id) in transaction_results {
                        transaction_query_result_repository.add_result(id, tx_id);
                    }
                    Ok(())
                })
                .wait()
                .unwrap();
        }

        assert!(
            harness
                .transaction_query_result_repository
                .get(harness.first_transaction_query_id)
                .is_some(),
            "Query not moved to result repository after matching transaction \
             requiring single confirmation arrived in block"
        );
    }

    #[test]
    fn given_double_confirmation_query_when_matching_transaction_is_processed_and_confirmed_adds_result(
    ) {
        let harness = Setup::new(1, 1, 2, 0, 0);
        let mut block_processor = harness.block_processor;

        {
            let block_query_result_repository = harness.block_query_result_repository.clone();
            let transaction_query_result_repository =
                harness.transaction_query_result_repository.clone();

            block_processor
                .process(harness.first_block)
                .and_then(move |(block_results, transaction_results)| {
                    for (id, block_id) in block_results {
                        block_query_result_repository.add_result(id, block_id);
                    }
                    for (id, tx_id) in transaction_results {
                        transaction_query_result_repository.add_result(id, tx_id);
                    }
                    Ok(())
                })
                .wait()
                .unwrap();
        }

        // Transaction not yet confirmed
        assert!(
            harness
                .transaction_query_result_repository
                .get(harness.first_transaction_query_id)
                .is_none(),
            "Query found in result repository even though matching transaction \
             still requires one more confirmation"
        );

        let empty_block = GenericBlock::default();
        block_processor.process(empty_block);

        // Transaction now has enough confirmation
        assert!(
            harness
                .transaction_query_result_repository
                .get(harness.first_transaction_query_id)
                .is_some(),
            "Query not moved to result repository after matching transaction \
             sufficiently confirmed"
        );
    }

    #[test]
    fn given_single_confirmation_query_when_non_matching_transaction_is_processed_does_not_add_result(
    ) {
        let harness = Setup::new(1, 2, 1, 0, 0);
        let mut block_processor = harness.block_processor;

        {
            let block_query_result_repository = harness.block_query_result_repository.clone();
            let transaction_query_result_repository =
                harness.transaction_query_result_repository.clone();

            block_processor
                .process(harness.first_block)
                .and_then(move |(block_results, transaction_results)| {
                    for (id, block_id) in block_results {
                        block_query_result_repository.add_result(id, block_id);
                    }
                    for (id, tx_id) in transaction_results {
                        transaction_query_result_repository.add_result(id, tx_id);
                    }
                    Ok(())
                })
                .wait()
                .unwrap();
        }

        assert!(
            harness
                .transaction_query_result_repository
                .get(harness.first_transaction_query_id)
                .is_none(),
            "Query moved to result repository after non-matching transaction \
             arrived in block"
        );
    }

    #[test]
    fn given_block_timestamp_query_when_younger_block_is_processed_add_result() {
        let harness = Setup::new(1, 2, 1, 5, 6);
        let mut block_processor = harness.block_processor;

        {
            let block_query_result_repository = harness.block_query_result_repository.clone();
            let transaction_query_result_repository =
                harness.transaction_query_result_repository.clone();

            block_processor
                .process(harness.first_block)
                .and_then(move |(block_results, transaction_results)| {
                    for (id, block_id) in block_results {
                        block_query_result_repository.add_result(id, block_id);
                    }
                    for (id, tx_id) in transaction_results {
                        transaction_query_result_repository.add_result(id, tx_id);
                    }
                    Ok(())
                })
                .wait()
                .unwrap();
        }

        assert!(
            harness
                .block_query_result_repository
                .get(harness.first_block_query_id)
                .is_some(),
            "Query moved to result repository after matching block arrived"
        );
    }

    #[test]
    fn given_block_timestamp_query_when_older_block_is_processed_does_not_add_result() {
        let harness = Setup::new(1, 2, 1, 6, 5);
        let mut block_processor = harness.block_processor;

        {
            let block_query_result_repository = harness.block_query_result_repository.clone();
            let transaction_query_result_repository =
                harness.transaction_query_result_repository.clone();

            block_processor
                .process(harness.first_block)
                .and_then(move |(block_results, transaction_results)| {
                    for (id, block_id) in block_results {
                        block_query_result_repository.add_result(id, block_id);
                    }
                    for (id, tx_id) in transaction_results {
                        transaction_query_result_repository.add_result(id, tx_id);
                    }
                    Ok(())
                })
                .wait()
                .unwrap();
        }

        assert!(
            harness
                .block_query_result_repository
                .get(harness.first_block_query_id)
                .is_none(),
            "Query not moved to result repository after non-matching block arrived"
        );
    }
}
