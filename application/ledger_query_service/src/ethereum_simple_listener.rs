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

// TODO: make it configurable as for production you may want to wait a bit longer
const POLLING_WAIT_TIME: Duration = Duration::from_secs(1);

pub struct EthereumSimpleListener<P> {
    _event_loop: EventLoopHandle,
    client: Web3<Http>,
    filter: BaseFilter<Http, H256>,
    processor: P,
}

#[derive(Debug)]
pub enum Error {
    Web3InitFail(web3::Error),
    FilterFail(web3::Error),
}

impl<P: TransactionProcessor<EthereumTransaction>> EthereumSimpleListener<P> {
    pub fn new(endpoint: &str, processor: P) -> Result<Self, Error> {
        let (event_loop, transport) = Http::new(&endpoint).map_err(Error::Web3InitFail)?;
        let client = Web3::new(transport);

        let filter = client.eth_filter();
        let filter = filter
            .create_blocks_filter()
            .wait()
            .map_err(Error::FilterFail)?;

        Ok(EthereumSimpleListener {
            _event_loop: event_loop,
            client,
            filter,
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
            .stream(POLLING_WAIT_TIME)
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
