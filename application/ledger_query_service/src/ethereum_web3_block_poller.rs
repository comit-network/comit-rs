use block_processor::BlockProcessor;
use std::time::Duration;
use web3::{
    self,
    api::BaseFilter,
    futures::{Future, Stream},
    transports::{EventLoopHandle, Http},
    types::{Block, BlockId, Transaction as EthereumTransaction, TransactionId, H256},
    Web3,
};

use futures::stream::iter_ok;

#[derive(DebugStub)]
pub struct EthereumWeb3BlockPoller<P> {
    #[debug_stub = "EventLoop"]
    _event_loop: EventLoopHandle,
    client: Web3<Http>,
    filter: BaseFilter<Http, H256>,
    polling_interval: Duration,
    #[debug_stub = "Processor"]
    processor: P,
}

impl<P: BlockProcessor<Block<EthereumTransaction>>> EthereumWeb3BlockPoller<P> {
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

        let (client, mut processor) = (self.client, self.processor);

        let result = self
            .filter
            .stream(self.polling_interval)
            .and_then(|block_hash| client.eth().block_with_txs(BlockId::from(block_hash)))
            .filter(Option::is_some)
            .map(Option::unwrap)
            .for_each(|block| {
                processor.process(&block);
                Ok(())
            }).wait();

        info!("Ethereum block polling has stopped: {:?}", result);
    }
}
