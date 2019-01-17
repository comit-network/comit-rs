use crate::{
    query_repository::QueryRepository, query_result_repository::QueryResultRepository,
    ArcQueryRepository, ArcQueryResultRepository, BlockProcessor, Query, QueryMatchResult,
};
use futures::{future::join_all, Future};
use std::{
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use tokio;

type QueryMatch = (u32, String);

pub trait Transaction: Debug + 'static + Clone {
    fn transaction_id(&self) -> String;
}

pub trait Block: Debug + 'static + Clone {
    type Transaction: Transaction;

    fn blockhash(&self) -> String;
    fn prev_blockhash(&self) -> String;
    fn transactions(&self) -> &[Self::Transaction];
}

#[derive(Debug)]
pub struct PendingTransaction {
    matching_query_id: u32,
    tx_id: String,
    pending_confirmations: u32,
}

#[derive(DebugStub)]
pub struct DefaultBlockProcessor<T, B, TQ, BQ> {
    #[debug_stub = "Queries"]
    transaction_queries: ArcQueryRepository<TQ>,
    #[debug_stub = "Queries"]
    block_queries: ArcQueryRepository<BQ>,
    #[debug_stub = "Results"]
    transaction_results: ArcQueryResultRepository<TQ>,
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
    ) -> Box<dyn Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()> + Send> {
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

        let block_results = Self::process_new_block(Arc::clone(&self.block_queries), &block);
        let mut tx_result_vecs = vec![];

        for tx in block.transactions() {
            tx_result_vecs.push(Self::process_new_transaction(
                Arc::clone(&self.transaction_queries),
                Arc::clone(&self.pending_transactions),
                tx,
            ))
        }

        let tx_results = join_all(tx_result_vecs)
            .map(|tx_result_vec| tx_result_vec.into_iter().flatten().collect());

        Box::new(block_results.join(tx_results))
    }
}

impl<T: Transaction, B: Block<Transaction = T>, TQ: Query<T> + 'static, BQ: Query<B>>
    DefaultBlockProcessor<T, B, TQ, BQ>
{
    fn process_new_block(
        block_queries: ArcQueryRepository<BQ>,
        block: &B,
    ) -> impl Future<Item = Vec<QueryMatch>, Error = ()> + Send {
        trace!("Processing {:?}", block);
        let block_id = block.blockhash();
        let mut query_match_futures = vec![];

        // We must collect the futures in a vector first to stop
        // borrow checker freaking out
        for (query_id, query) in block_queries.all() {
            trace!("Matching query {:#?} against block {:#?}", query, block);
            let block_id = block_id.clone();
            let result_future = query.matches(block).map(move |result| match result {
                QueryMatchResult::Yes { .. } => {
                    trace!("Block {} matches Query-ID: {:?}", block_id, query_id);
                    Some((query_id, block_id))
                }
                QueryMatchResult::No => None,
            });
            query_match_futures.push(result_future);
        }

        join_all(query_match_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
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
        transaction: &T,
    ) -> impl Future<Item = Vec<QueryMatch>, Error = ()> + Send {
        trace!("Processing {:?}", transaction);
        let mut result_futures = vec![];

        for (query_id, query) in transaction_queries.all() {
            trace!(
                "Matching query {:#?} against transaction {:#?}",
                query,
                transaction
            );

            let tx_id = transaction.transaction_id();
            let pending_transactions = Arc::clone(&pending_transactions);

            let result_future =
                query.matches(transaction).map(
                    move |query_match_result| match query_match_result {
                        QueryMatchResult::Yes {
                            confirmations_needed: 0,
                        }
                        | QueryMatchResult::Yes {
                            confirmations_needed: 1,
                        } => {
                            trace!(
                                "Confirmed transaction {} matches Query-ID: {:?}",
                                tx_id,
                                query_id
                            );
                            Some((query_id, tx_id))
                        }
                        QueryMatchResult::Yes {
                            confirmations_needed,
                        } => {
                            trace!(
                                "Unconfirmed transaction {} matches Query-ID: {:?}",
                                tx_id,
                                query_id
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
                );
            result_futures.push(result_future);
        }

        join_all(result_futures).map(|results| results.into_iter().filter_map(|x| x).collect())
    }
}

impl<T, B, TQ, BQ> DefaultBlockProcessor<T, B, TQ, BQ> {
    pub fn new(
        transaction_query_repository: Arc<dyn QueryRepository<TQ>>,
        block_query_repository: Arc<dyn QueryRepository<BQ>>,
        transaction_query_result_repository: Arc<dyn QueryResultRepository<TQ>>,
    ) -> Self {
        Self {
            transaction_queries: transaction_query_repository,
            block_queries: block_query_repository,
            transaction_results: transaction_query_result_repository,
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
        in_memory_query_result_repository::InMemoryQueryResultRepository, IsEmpty,
    };
    use spectral::prelude::*;

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
    }
    impl IsEmpty for GenericTransactionQuery {
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
    }
    impl IsEmpty for GenericBlockQuery {
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
        block_processor: DefaultBlockProcessor<
            GenericTransaction,
            GenericBlock,
            GenericTransactionQuery,
            GenericBlockQuery,
        >,
        first_block: GenericBlock,
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

            let block_processor = DefaultBlockProcessor::new(
                transaction_query_repository.clone(),
                block_query_repository.clone(),
                transaction_query_result_repository.clone(),
            );

            let first_transaction_query = GenericTransactionQuery {
                transaction_id: query_transaction_id,
                confirmations_needed,
            };

            transaction_query_repository
                .save(first_transaction_query)
                .unwrap();

            let first_transaction = GenericTransaction { id: transaction_id };

            let first_block_query = GenericBlockQuery {
                min_timestamp_secs: query_timestamp,
            };

            block_query_repository.save(first_block_query).unwrap();
            let first_block = GenericBlock {
                id: 0,
                parent_id: 0,
                timestamp: block_timestamp,
                transaction_list: vec![first_transaction],
            };

            Self {
                block_processor,
                first_block,
            }
        }
    }

    #[test]
    fn given_single_confirmation_query_when_matching_transaction_processes_returns_1_block_0_tx() {
        let harness = Setup::new(1, 1, 1, 0, 0);
        let mut block_processor = harness.block_processor;

        let (blocks, transactions) = process_results(block_processor.process(harness.first_block));

        assert_that(&blocks).named(&"found blocks").has_length(1);
        assert_that(&transactions).named(&"found txs").has_length(1);
    }

    #[test]
    #[ignore] // TODO fixme, pending transactions does not work correctly: https://github.com/comit-network/comit-rs/issues/591
    fn given_double_confirmation_query_when_matching_transaction_is_processed_and_confirmed_adds_result(
    ) {
        let harness = Setup::new(1, 1, 2, 0, 0);
        let mut block_processor = harness.block_processor;

        let (blocks, transactions) = process_results(block_processor.process(harness.first_block));
        assert_that(&blocks).named(&"found blocks").has_length(1);
        // Transaction not yet confirmed
        assert_that(&transactions).named(&"found txs").has_length(0);

        let empty_block = GenericBlock::default();
        let (blocks, transactions) = process_results(block_processor.process(empty_block));
        assert_that(&blocks).named(&"found blocks").has_length(1);
        assert_that(&transactions).named(&"found txs").has_length(1);
    }

    #[test]
    fn given_single_confirmation_query_when_non_matching_transaction_process_returns_1_block_0_tx()
    {
        let harness = Setup::new(1, 2, 1, 0, 0);
        let mut block_processor = harness.block_processor;

        let (blocks, transactions) = process_results(block_processor.process(harness.first_block));
        assert_that(&blocks).named(&"found blocks").has_length(1);
        assert_that(&transactions).named(&"found txs").has_length(0);
    }

    #[test]
    fn given_block_timestamp_query_when_younger_block_process_returns_1_block_0_tx() {
        let harness = Setup::new(1, 2, 1, 5, 6);
        let mut block_processor = harness.block_processor;

        let (blocks, transactions) = process_results(block_processor.process(harness.first_block));
        assert_that(&blocks).named(&"found blocks").has_length(1);
        assert_that(&transactions).named(&"found txs").has_length(0);
    }

    #[test]
    fn given_block_timestamp_query_when_older_block_process_returns_0_block_0_tx() {
        let harness = Setup::new(1, 2, 1, 6, 5);
        let mut block_processor = harness.block_processor;

        let (blocks, transactions) = process_results(block_processor.process(harness.first_block));

        assert_that(&blocks).named(&"found blocks").has_length(0);
        assert_that(&transactions).named(&"found txs").has_length(0);
    }

    fn process_results(
        processing_future: Box<
            dyn Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()> + Send,
        >,
    ) -> (Vec<(u32, String)>, Vec<(u32, String)>) {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(processing_future).unwrap()
    }
}
