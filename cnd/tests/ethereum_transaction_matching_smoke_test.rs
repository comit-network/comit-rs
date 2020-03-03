use chrono::offset::Utc;
use cnd::{
    btsieve::ethereum::{matching_transaction_and_receipt, Web3Connector},
    ethereum::{U256},
};
use futures_core::compat::Future01CompatExt;
use reqwest::Url;
use std::time::Duration;
use testcontainers::*;
use web3::{
    transports::{EventLoopHandle, Http},
    Web3,
    types::TransactionRequest
};

/// A very basic e2e test that verifies that we glued all our code together
/// correctly for ethereum transaction pattern matching.
///
/// We get the default account from the node and send some money to it
/// from the parity dev account. Afterwards we verify that the tx hash of
/// the sent tx equals the one that we found through btsieve.
#[tokio::test]
async fn ethereum_transaction_matching_smoke_test() {
    let cli = clients::Cli::default();
    let container = cli.run(images::parity_parity::ParityEthereum::default());
    let start_of_swap = Utc::now().naive_local();

    let (_handle, client) = new_web3_client(&container);

    let mut url = Url::parse("http://localhost").expect("failed to parse static URL");
    #[allow(clippy::cast_possible_truncation)]
    url.set_port(Some(
        container
            .get_host_port(8545)
            .expect("failed to get host port") as u16,
    ))
    .unwrap();

    let connector = Web3Connector::new(url);

    let accounts = client
        .eth()
        .accounts()
        .compat()
        .await
        .expect("failed to get accounts");

    let target_address = accounts[0];

    let send_money_to_address = async {
        tokio::time::delay_for(Duration::from_secs(2)).await;
        client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: "00a329c0648769a73afac7f9381e08fb43dbea72"
                        .parse()
                        .expect("failed to parse static string"),
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
            .expect("failed to send transaction")
    };
    let transaction = tokio::time::timeout(Duration::from_secs(5), send_money_to_address)
        .await
        .expect("failed to send money to address");

    let (matched_transaction, _receipt) = tokio::time::timeout(
        Duration::from_secs(5),
        matching_transaction_and_receipt(connector, start_of_swap, |transaction| {
            transaction.to == Some(target_address)
        }),
    )
    .await
    .expect("failed to timeout")
    .expect("failed to get the actual transaction and receipt");

    assert_eq!(matched_transaction.hash, transaction)
}

pub fn new_web3_client<D, E>(container: &Container<'_, D, E>) -> (EventLoopHandle, Web3<Http>)
where
    D: Docker,
    E: Image,
{
    let port = container.get_host_port(8545).unwrap();
    let endpoint = format!("http://localhost:{}", port);

    let (event_loop, transport) = Http::new(&endpoint).unwrap();
    let web3 = Web3::new(transport);

    (event_loop, web3)
}
