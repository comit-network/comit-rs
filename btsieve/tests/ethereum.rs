use btsieve::{
    ethereum::{web3_http_blocksource::Web3HttpBlockSource, TransactionQuery},
    matching_transactions::MatchingTransactions,
};
use ethereum_support::{TransactionRequest, U256};
use futures::{Future, Stream};
use reqwest::Url;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use testcontainers::*;
use tokio::{prelude::FutureExt, runtime::Runtime, timer::Delay};

/// A very basic e2e test that verifies that we glued all our code together
/// correctly for ethereum queries
///
/// We get the default account from the node and send some money to it
/// from the parity dev account. Afterwards we verify that the tx hash of
/// the sent tx equals the one that we found through btsieve.
#[test]
fn ethereum_transaction_query_e2e_test() {
    let cli = clients::Cli::default();
    let container = cli.run(images::parity_parity::ParityEthereum::default());

    let (_handle, client) = tc_web3_client::new(&container);

    let mut url = Url::parse("http://localhost").unwrap();
    #[allow(clippy::cast_possible_truncation)]
    url.set_port(Some(container.get_host_port(8545).unwrap() as u16))
        .unwrap();

    let blocksource =
        Arc::new(Web3HttpBlockSource::new(url, ethereum_support::Network::Regtest).unwrap());

    let mut runtime = Runtime::new().unwrap();

    let accounts = runtime.block_on(client.eth().accounts()).unwrap();

    let target_address = accounts[0];

    let funding_transaction = blocksource
        .matching_transactions(TransactionQuery {
            from_address: None,
            to_address: Some(target_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
        })
        .take(1)
        .into_future()
        .map_err(|_| ());

    let now_in_two_seconds = Instant::now() + Duration::from_secs(2);

    let send_money_to_address = Delay::new(now_in_two_seconds)
        .map_err(|_| ())
        .and_then(move |_| {
            client
                .personal()
                .send_transaction(
                    TransactionRequest {
                        from: "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap(),
                        to: Some(target_address),
                        gas: None,
                        gas_price: None,
                        value: Some(U256::from(1_000_000_000)),
                        data: None,
                        nonce: None,
                        condition: None,
                    },
                    "",
                )
                .map_err(|_| ())
        })
        .map_err(|_| ());

    let future = send_money_to_address.join(funding_transaction);

    let future_with_timeout = future.timeout(Duration::from_secs(5));

    let (actual_transaction, (funding_transaction, _)) =
        runtime.block_on(future_with_timeout).unwrap();

    assert_eq!(funding_transaction.unwrap().hash, actual_transaction)
}
