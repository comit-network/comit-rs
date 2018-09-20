use ethereum_support::web3::{
    self,
    api::BaseFilter,
    futures::{Future, Stream},
    transports::{EventLoopHandle, Http},
    types::{BlockId, H256, Transaction as EthereumTransaction, TransactionId},
    Web3,
};
use std::time::Duration;
use transaction_processor::TransactionProcessor;

use futures::stream::iter_ok;

pub struct EthereumWeb3BlockPoller<P> {
    _event_loop: EventLoopHandle,
    client: Web3<Http>,
    filter: BaseFilter<Http, H256>,
    polling_interval: Duration,
    processor: P,
}

impl<P: TransactionProcessor<EthereumTransaction>> EthereumWeb3BlockPoller<P> {
    pub fn new(
        endpoint: &str,
        polling_wait_time: Duration,
        processor: P,
    ) -> Result<Self, web3::Error> {
        let (event_loop, transport) = Http::new(&endpoint)?;
        let client = Web3::new(transport);

        let filter = client.eth_filter();
        let filter = filter.create_blocks_filter().wait()?;

        Ok(EthereumWeb3BlockPoller {
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
            .and_then(|block_hash| client.eth().block(BlockId::from(block_hash)))
            .filter(Option::is_some)
            .map(|block| iter_ok(block.unwrap().transactions))
            .flatten()
            .map(TransactionId::Hash)
            .and_then(|transaction_id| client.eth().transaction(transaction_id))
            .filter(Option::is_some)
            .map(Option::unwrap)
            .for_each(|transaction| Ok(processor.process(&transaction)))
            .wait();

        info!("Ethereum block polling has stopped: {:?}", result);
    }
}
