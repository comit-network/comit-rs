use btsieve::{blocksource::BlockSource, ethereum::web3_http_blocksource::Web3HttpBlockSource};
use ethereum_support::{
    web3::{transports::Http, Web3},
    TransactionRequest, H256, U256,
};
use failure::_core::time::Duration;
use futures::{Future, Stream};
use std::sync::Arc;
use testcontainers::Docker;
use tokio::{prelude::StreamExt, runtime::Runtime};

#[test]
fn io_errors_will_not_stop_the_stream() {
    let cli = testcontainers::clients::Cli::default();
    let mut runtime = Runtime::new().unwrap();

    let container = cli.run(testcontainers::images::parity_parity::ParityEthereum::default());
    let (_handle, web3) = tc_web3_client::new(&container);
    let web3 = Arc::new(web3);

    let blocksource = runtime
        .block_on(Web3HttpBlockSource::new(web3.clone()))
        .unwrap();
    let blocks = blocksource.blocks();

    runtime.block_on(create_a_block(web3.as_ref())).unwrap();
    runtime.block_on(create_a_block(web3.as_ref())).unwrap();

    // shutdown the container, which will trigger an IO error in our blocksource
    std::mem::drop(container);

    // we try to take 3 blocks from the stream but we only created 2 blocks

    // we killed the container, so there will never be a third one
    // the blocksource is still polling the parity node, so after the node being
    // dead, we will perceive IO an error we expect the IO error happening
    // inside the blocksource to not be propagated and hence, we expect the
    // stream to terminate with the timeout of 5 seconds
    let blocks = blocks.take(3).timeout(Duration::from_secs(5)).collect();
    let blocks = runtime.block_on(blocks);

    let error = blocks.unwrap_err();

    // if we are elapsed, we successfully avoided the error from inside the
    // BlockSource but failed because we reached the 5 second timeout
    assert_eq!(error.is_elapsed(), true)
}

fn create_a_block(
    web3: &Web3<Http>,
) -> impl Future<Item = H256, Error = ethereum_support::web3::Error> + Send {
    // triggering a transaction will create a block in parity dev mode
    web3.personal().send_transaction(
        TransactionRequest {
            // parity dev account
            from: "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap(),
            to: Some("0000000000000000000000000000000000000000".parse().unwrap()),
            gas: None,
            gas_price: None,
            value: Some(U256::from(1_000_000)),
            data: None,
            nonce: None,
            condition: None,
        },
        "",
    )
}
