use chrono::offset::Utc;
use comit::{
    btsieve::ethereum::{matching_transaction, TransactionPattern, Web3Connector},
    ethereum::{TransactionRequest, U256},
};
use futures_core::compat::Future01CompatExt;
use reqwest::Url;
use std::time::Duration;
use testcontainers::*;
use web3::{
    transports::{EventLoopHandle, Http},
    Web3,
};

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

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(target_address),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };
    let matched_transaction = tokio::time::timeout(
        Duration::from_secs(5),
        matching_transaction(connector, pattern, start_of_swap),
    )
    .await
    .expect("failed to timeout");

    assert_eq!(
        matched_transaction
            .expect("failed to get funding transaction")
            .transaction
            .hash,
        transaction
    )
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
