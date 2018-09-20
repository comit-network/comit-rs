use std::time::Duration;
use transaction_processor::TransactionProcessor;
use web3::{
    self,
    api::BaseFilter,
    futures::{Future, Stream},
    transports::{EventLoopHandle, Http},
    types::{BlockId, H256, Transaction as EthereumTransaction, TransactionId},
    Web3,
};

pub struct EthereumSimpleListener<P> {
    _event_loop: EventLoopHandle,
    client: Web3<Http>,
    filter: BaseFilter<Http, H256>,
    polling_interval: Duration,
    processor: P,
}

impl<P: TransactionProcessor<EthereumTransaction>> EthereumSimpleListener<P> {
    pub fn new(
        endpoint: &str,
        polling_wait_time: Duration,
        processor: P,
    ) -> Result<Self, web3::Error> {
        let (event_loop, transport) = Http::new(&endpoint)?;
        let client = Web3::new(transport);

        let filter = client.eth_filter();
        let filter = filter.create_blocks_filter().wait()?;

        Ok(EthereumSimpleListener {
            _event_loop: event_loop,
            client,
            filter,
            polling_interval: polling_wait_time,
            processor,
        })
    }

    pub fn start(self) {
        info!(
            "Starting listener for Ethereum from block {} waiting for new blocks.",
            self.client
                .eth()
                .block_number()
                .wait()
                .expect("Could not get block height from web3 client")
        );

        let (client, processor) = (self.client, self.processor);

        let result = self
            .filter
            .stream(self.polling_interval)
            .for_each(move |block_hash| {
                let block_id = BlockId::from(block_hash);

                //TODO: remove some unwraps
                let block = client.eth().block(block_id).wait().unwrap().unwrap();

                for &transaction_hash in block.transactions.iter() {
                    let transaction_id = TransactionId::Hash(transaction_hash);
                    //TODO: remove some unwraps
                    let transaction = client
                        .eth()
                        .transaction(transaction_id)
                        .wait()
                        .unwrap()
                        .unwrap();
                    processor.process(&transaction);
                }
                Ok(())
            })
            .wait();
        info!("Ethereum block polling has stopped: {:?}", result);
    }
}
