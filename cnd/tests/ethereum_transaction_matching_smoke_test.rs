use chrono::offset::Utc;
use cnd::{
    btsieve::ethereum::{matching_transaction_and_receipt, Web3Connector},
    ethereum::{Address, U256},
    jsonrpc,
};
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
async fn ethereum_transaction_matching_smoke_test() {
    let cli = clients::Cli::default();
    let container = cli.run(images::parity_parity::ParityEthereum::default());
    let start_of_swap = Utc::now().naive_local();

    let url = connection_url(&container).unwrap();

    let client = jsonrpc::Client::new(url.clone());
    let connector = Web3Connector::new(url);

    let accounts: Vec<Address> = client
        .send(jsonrpc::Request::new("eth_accounts", Vec::<u32>::new()))
        .await
        .expect("failed to get accounts");

    let target_address = accounts[0];

    let send_money_to_address = async {
        tokio::time::delay_for(Duration::from_secs(2)).await;
        client
            .send(jsonrpc::Request::new("personal_sendTransaction", vec![
                jsonrpc::serialize(TransactionRequest {
                    from: "00a329c0648769a73afac7f9381e08fb43dbea72"
                        .parse()
                        .expect("failed to parse static string"),
                    to: target_address,
                    value: U256::from(1_000_000_000u64),
                })
                .unwrap(),
                jsonrpc::serialize("").unwrap(),
            ]))
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

fn connection_url<D, E>(container: &Container<'_, D, E>) -> anyhow::Result<reqwest::Url>
where
    D: Docker,
    E: Image,
{
    let port = container.get_host_port(8545).unwrap();
    let endpoint = format!("http://localhost:{}", port);

    let url = Url::parse(&endpoint)?;

    Ok(url)
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct TransactionRequest {
    pub from: Address,
    pub to: Address,
    pub value: U256,
}
