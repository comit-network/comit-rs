use crate::{
    blocksource::{BlockSource, Error},
    web3::{
        self,
        futures::{Future, Stream},
        transports::Http,
        types::{Block, BlockId, Transaction},
        Web3,
    },
};
use ethereum_support::Network;
use std::{sync::Arc, time::Duration};
use tokio::timer::Interval;
use web3::types::BlockNumber;

pub struct Web3HttpBlockSource {
    web3: Arc<Web3<Http>>,
    network: Network,
}

impl Web3HttpBlockSource {
    pub fn new(web3: Arc<Web3<Http>>) -> impl Future<Item = Self, Error = web3::Error> {
        web3.clone().net().version().map(move |version| Self {
            web3,
            network: Network::from_network_id(version),
        })
    }
}

impl BlockSource for Web3HttpBlockSource {
    type Block = Block<Transaction>;
    type Error = web3::Error;

    fn blocks(&self) -> Box<dyn Stream<Item = Self::Block, Error = Error<Self::Error>> + Send> {
        let web = self.web3.clone();

        let poll_interval = match self.network {
            Network::Mainnet => 5000,
            Network::Ropsten => 5000,
            Network::Regtest => 500,
            Network::Unknown => 1000,
        };

        log::info!(target: "ethereum::blocksource", "polling for new blocks on {} every {} miliseconds", self.network, poll_interval);

        let stream = Interval::new_interval(Duration::from_millis(poll_interval))
            .map_err(Error::Timer)
            .and_then(move |_| {
                web.eth()
                    .block_with_txs(BlockId::Number(BlockNumber::Latest))
                    .or_else(|error| {
                        match error {
                            web3::Error::Io(e) => {
                                log::debug!(target: "ethereum::blocksource", "IO error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            web3::Error::Transport(e)  => {
                                log::debug!(target: "ethereum::blocksource", "Transport error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            _ => Err(error)
                        }
                    })
                    .map_err(Error::Source)
            })
            .filter_map(|maybe_block| maybe_block)
            .inspect(|block| {
                if let Block { hash: Some(hash), number: Some(number), .. } = block {
                    log::trace!(target: "ethereum::blocksource", "latest block is {:?} at height {}", hash, number);
                }
            });

        Box::new(stream)
    }
}
