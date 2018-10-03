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

pub struct UnconfirmedMatchingTransaction {
    query_id: u32,
    tx_id: String,
    confirmations_still_needed: u32,
}

#[derive(DebugStub)]
pub struct DefaultBlockProcessor<T, Q> {
    #[debug_stub = "Queries"]
    queries: Arc<QueryRepository<Q>>,
    #[debug_stub = "Results"]
    results: Arc<QueryResultRepository<Q>>,
    unconfirmed_matching_transactions: Vec<UnconfirmedMatchingTransaction>,
    blockhashes: Vec<String>,
    tx_type: PhantomData<T>,
}

impl<T: Transaction, B: Block<Transaction = T>, Q: Query<T> + 'static> BlockProcessor<B>
    for DefaultBlockProcessor<T, Q>
{
    fn process(&mut self, block: &B) {
        trace!("New block received: {:?}", block);

        match self.blockhashes.last() {
            Some(last_blockhash) => {
                if *last_blockhash != block.prev_blockhash() {
                    warn!(
                        "Last blockhash in chain doesn't match with block {} previous blockhash",
                        block.blockhash()
                    );
                }
            }
            None => (),
        }

        self.blockhashes.push(block.blockhash());

        self.update_unconfirmed_txs_queue();

        block
            .transactions()
            .iter()
            .for_each(|tx| self.process_new_tx(tx));
    }
}

impl<T: Transaction, Q: Query<T> + 'static> DefaultBlockProcessor<T, Q> {
    fn update_unconfirmed_txs_queue(&mut self) {
        trace!("Updating unconfirmed transaction queue");
        self.unconfirmed_matching_transactions
            .iter_mut()
            .for_each(|utx| utx.confirmations_still_needed -= 1);

        self.unconfirmed_matching_transactions
            .iter()
            .for_each(|utx| {
                if utx.confirmations_still_needed <= 0 {
                    trace!(
                        "Sending newly confirmed transaction {:?} to query result repository",
                        utx.tx_id
                    );
                    self.results
                        .add_result(utx.query_id.clone(), utx.tx_id.clone())
                }
            });

        self.unconfirmed_matching_transactions
            .retain(|ref utx| utx.confirmations_still_needed > 0);
    }

    fn process_new_tx(&mut self, transaction: &T) {
        trace!("Processing {:?}", transaction);

        self.queries
            .all()
            .map(|(id, query)| {
                trace!(
                    "Matching query {:#?} against transaction {:#?}",
                    query,
                    transaction
                );

                let tx_matches = query.matches(transaction);

                if tx_matches {
                    let is_confirmed = query.confirmations_needed() <= 1;
                    let tx_id = transaction.transaction_id();

                    if is_confirmed {
                        info!(
                            "Transaction {} matches Query-ID: {:?}",
                            transaction.transaction_id(),
                            id
                        );
                        (true, Some((id, tx_id)), None)
                    } else {
                        (
                            false,
                            None,
                            Some(UnconfirmedMatchingTransaction {
                                query_id: id,
                                tx_id,
                                confirmations_still_needed: query.confirmations_needed() - 1,
                            }),
                        )
                    }
                } else {
                    (false, None, None)
                }
            }).for_each(|(is_result_ready, result, queue_entry)| {
                match (is_result_ready, result, queue_entry) {
                    (true, Some((query_id, tx_id)), None) => {
                        self.results.add_result(query_id, tx_id)
                    }
                    (false, None, Some(unconfirmed_matching_tx)) => {
                        self.unconfirmed_matching_transactions
                            .push(unconfirmed_matching_tx);
                    }
                    _ => (),
                }
            })
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
            unconfirmed_matching_transactions: Vec::new(),
            blockhashes: Vec::new(),
            tx_type: PhantomData,
        }
    }
}
