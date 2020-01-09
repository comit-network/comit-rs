use cnd::{
    btsieve::ethereum::{matching_transaction, TransactionPattern, Web3Connector},
    ethereum::{
        web3::{
            transports::{EventLoopHandle, Http},
            Web3,
        },
        TransactionRequest, U256,
    },
};
use futures_core::{compat::Future01CompatExt, future};
use reqwest::Url;
use std::time::Duration;
use testcontainers::*;

/// A very basic e2e test that verifies that we glued all our code together
/// correctly for ethereum transaction pattern matching.
///
/// We get the default account from the node and send some money to it
/// from the parity dev account. Afterwards we verify that the tx hash of
/// the sent tx equals the one that we found through btsieve.
#[tokio::test]
async fn ethereum_transaction_pattern_e2e_test() {
    let cli = clients::Cli::default();
    let container = cli.run(images::parity_parity::ParityEthereum::default());

    let (_handle, client) = new_web3_client(&container);

    let mut url = Url::parse("http://localhost").unwrap();
    #[allow(clippy::cast_possible_truncation)]
    url.set_port(Some(container.get_host_port(8545).unwrap() as u16))
        .unwrap();

    let (connector, _event_loop) = Web3Connector::new(url).unwrap();

    let accounts = client.eth().accounts().compat().await.unwrap();

    let target_address = accounts[0];

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(target_address),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };
    let funding_transaction = matching_transaction(connector, pattern, None);
    let send_money_to_address = async {
        tokio::time::delay_for(Duration::from_secs(2)).await;
        client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap(),
                    to: Some(target_address),
                    gas: None,
                    gas_price: None,
                    value: Some(U256::from(1_000_000_000u64)),
                    data: None,
                    nonce: None,
                    condition: None,
                },
                "",
            )
            .compat()
            .await
            .unwrap()
    };

    let future = future::join(send_money_to_address, funding_transaction);

    let (actual_transaction, funding_transaction) =
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .unwrap();

    assert_eq!(funding_transaction.transaction.hash, actual_transaction)
}

pub fn new_web3_client<D: Docker, E: Image>(
    container: &Container<'_, D, E>,
) -> (EventLoopHandle, Web3<Http>) {
    let port = container.get_host_port(8545).unwrap();
    let endpoint = format!("http://localhost:{}", port);

    let (event_loop, transport) = Http::new(&endpoint).unwrap();
    let web3 = Web3::new(transport);

    (event_loop, web3)
}
