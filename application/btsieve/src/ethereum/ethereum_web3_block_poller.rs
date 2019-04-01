use crate::web3::{
	self,
	futures::{Future, Stream},
	transports::Http,
	types::{Block, BlockId, Transaction},
	Web3,
};
use std::{sync::Arc, time::Duration};

pub fn ethereum_block_listener(
	client: Arc<Web3<Http>>,
	polling_wait_time: Duration,
) -> Result<Box<dyn Stream<Item = Block<Transaction>, Error = ()> + Send>, web3::Error> {
	let filter = client.eth_filter().create_blocks_filter().wait()?;

	info!(
		"Starting listener for Ethereum from block {} waiting for new blocks.",
		client
			.eth()
			.block_number()
			.wait()
			.expect("Could not get block height from web3 client")
	);

	Ok(Box::new(
		filter
			.stream(polling_wait_time)
			.and_then(move |block_hash| client.eth().block_with_txs(BlockId::from(block_hash)))
			.filter_map(|item| item)
			.map_err(|error| error!("Could not read block: {:?}", error)),
	))
}
