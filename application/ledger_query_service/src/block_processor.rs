use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

pub trait BlockProcessor<B> {
    fn process(&mut self, block: &B);
}

pub trait Transaction: Debug {
    fn transaction_id(&self) -> String;
}

pub trait Block: Debug {
    type Transaction: Transaction;

    fn blockhash(&self) -> String;
    fn prev_blockhash(&self) -> String;
    fn transactions(&self) -> &[Self::Transaction];
}

pub trait Query<T>: Debug {
    fn matches(&self, transaction: &T) -> bool;
    fn confirmations_needed(&self) -> u32;
}

#[derive(Debug)]
pub struct PendingTransaction {
    matching_query_id: u32,
    tx_id: String,
    pending_confirmations: u32,
}

#[derive(DebugStub)]
pub struct DefaultBlockProcessor<T, Q> {
    #[debug_stub = "Queries"]
    queries: Arc<QueryRepository<Q>>,
    #[debug_stub = "Results"]
    results: Arc<QueryResultRepository<Q>>,
    pending_transactions: Vec<PendingTransaction>,
    blockhashes: Vec<String>,
    tx_type: PhantomData<T>,
}

impl<T: Transaction, B: Block<Transaction = T>, Q: Query<T> + 'static> BlockProcessor<B>
    for DefaultBlockProcessor<T, Q>
{
    fn process(&mut self, block: &B) {
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

        block
            .transactions()
            .iter()
            .for_each(|tx| self.process_new_transaction(tx));
    }
}

impl<T: Transaction, Q: Query<T> + 'static> DefaultBlockProcessor<T, Q> {
    fn update_pending_transactions(&mut self) {
        trace!("Updating pending matching transactions");
        self.pending_transactions
            .iter_mut()
            .for_each(|utx| utx.pending_confirmations -= 1);

        self.pending_transactions.iter().for_each(|utx| {
            if utx.pending_confirmations == 0 {
                let confirmed_tx_id = &utx.tx_id;
                trace!(
                    "Transaction {} now has enough confirmations. Sent to query result repository",
                    confirmed_tx_id
                );
                self.results
                    .add_result(utx.matching_query_id, confirmed_tx_id.clone())
            }
        });

        self.pending_transactions
            .retain(|ref utx| utx.pending_confirmations > 0);
    }

    fn process_new_transaction(&mut self, transaction: &T) {
        trace!("Processing {:?}", transaction);

        let result_repository = &mut self.results;
        let pending_transactions = &mut self.pending_transactions;
        self.queries
            .all()
            .filter_map(|(query_id, query)| {
                trace!(
                    "Matching query {:#?} against transaction {:#?}",
                    query,
                    transaction
                );

                if query.matches(transaction) {
                    let tx_id = transaction.transaction_id();
                    let query_tx_confirmations_needed = query.confirmations_needed();

                    if query_tx_confirmations_needed <= 1 {
                        info!(
                            "Confirmed transaction {} matches Query-ID: {:?}",
                            tx_id, query_id
                        );
                        Some((query_id, tx_id))
                    } else {
                        info!(
                            "Unconfirmed transaction {} matches Query-ID: {:?}",
                            tx_id, query_id
                        );
                        let pending_tx = PendingTransaction {
                            matching_query_id: query_id,
                            tx_id,
                            pending_confirmations: query_tx_confirmations_needed - 1,
                        };
                        pending_transactions.push(pending_tx);
                        None
                    }
                } else {
                    None
                }
            }).for_each(|(query_id, tx_id)| result_repository.add_result(query_id, tx_id))
    }
}

impl<T, Q> DefaultBlockProcessor<T, Q> {
    pub fn new(
        query_repository: Arc<QueryRepository<Q>>,
        query_result_repository: Arc<QueryResultRepository<Q>>,
    ) -> Self {
        Self {
            queries: query_repository,
            results: query_result_repository,
            pending_transactions: Vec::new(),
            blockhashes: Vec::new(),
            tx_type: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use in_memory_query_repository::InMemoryQueryRepository;
    use in_memory_query_result_repository::InMemoryQueryResultRepository;

    #[derive(Serialize, Deserialize, Clone, Default, Debug, Copy)]
    struct GenericQuery {
        transaction_id: u8,
        confirmations_needed: u32,
    }

    impl Query<GenericTransaction> for GenericQuery {
        fn matches(&self, transaction: &GenericTransaction) -> bool {
            self.transaction_id == transaction.id
        }

        fn confirmations_needed(&self) -> u32 {
            self.confirmations_needed
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

    #[derive(Debug, Default)]
    struct GenericBlock {
        id: u8,
        parent_id: u8,
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
        query_result_repository: Arc<InMemoryQueryResultRepository<GenericQuery>>,
        block_processor: DefaultBlockProcessor<GenericTransaction, GenericQuery>,
        first_query_id: u32,
        first_block: GenericBlock,
    }

    impl Setup {
        fn new(query_transaction_id: u8, transaction_id: u8, confirmations_needed: u32) -> Self {
            let query_repository = Arc::new(InMemoryQueryRepository::default());
            let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
            let block_processor = DefaultBlockProcessor::new(
                query_repository.clone(),
                query_result_repository.clone(),
            );

            let first_query = GenericQuery {
                transaction_id: query_transaction_id,
                confirmations_needed,
            };

            let first_query_id = query_repository.save(first_query).unwrap();

            let first_transaction = GenericTransaction { id: transaction_id };

            let first_block = GenericBlock {
                id: 0,
                parent_id: 0,
                transaction_list: vec![first_transaction],
            };

            Self {
                query_result_repository,
                block_processor,
                first_query_id,
                first_block,
            }
        }
    }
    #[test]
    fn given_single_confirmation_query_when_matching_transaction_is_processed_adds_result() {
        let harness = Setup::new(1, 1, 1);
        let mut block_processor = harness.block_processor;

        block_processor.process(&harness.first_block);

        assert!(
            harness
                .query_result_repository
                .get(harness.first_query_id)
                .is_some(),
            "Query not moved to result repository after matching transaction \
             requiring single confirmation arrived in block"
        );
    }

    #[test]
    fn given_double_confirmation_query_when_matching_transaction_is_processed_and_confirmed_adds_result(
) {
        let harness = Setup::new(1, 1, 2);
        let mut block_processor = harness.block_processor;

        block_processor.process(&harness.first_block);

        // Transaction not yet confirmed
        assert!(
            harness
                .query_result_repository
                .get(harness.first_query_id)
                .is_none(),
            "Query found in result repository even though matching transaction \
             still requires one more confirmation"
        );

        let empty_block = GenericBlock::default();
        block_processor.process(&empty_block);

        // Transaction now has enough confirmation
        assert!(
            harness
                .query_result_repository
                .get(harness.first_query_id)
                .is_some(),
            "Query not moved to result repository after matching transaction \
             sufficiently confirmed"
        );
    }
    #[test]
    fn given_single_confirmation_query_when_non_matching_transaction_is_processed_does_not_add_result(
) {
        let harness = Setup::new(1, 2, 1);
        let mut block_processor = harness.block_processor;

        block_processor.process(&harness.first_block);

        assert!(
            harness
                .query_result_repository
                .get(harness.first_query_id)
                .is_none(),
            "Query moved to result repository after non-matching transaction \
             arrived in block"
        );
    }
}
