use bitcoin::{Amount, Network};
use bitcoincore_rpc::RpcApi;
use chrono::offset::Utc;
use comit::btsieve::bitcoin::{matching_transaction, BitcoindConnector, TransactionPattern};
use futures_core::future;
use images::coblox_bitcoincore::BitcoinCore;
use reqwest::Url;
use std::time::Duration;
use testcontainers::*;

/// A very basic e2e test that verifies that we glued all our code together
/// correctly for bitcoin transaction pattern matching.
///
/// We send money to an address and check if the transaction that we filter out
/// is the same one as the one that was returned when we sent the money.
#[tokio::test]
async fn bitcoin_transaction_pattern_e2e_test() {
    let cli = clients::Cli::default();
    let container = cli.run(BitcoinCore::default());
    let client = new_bitcoincore_client(&container);

    let mut url = Url::parse("http://localhost").unwrap();
    #[allow(clippy::cast_possible_truncation)]
    url.set_port(Some(container.get_host_port(18443).unwrap() as u16))
        .unwrap();

    let connector = BitcoindConnector::new(url, Network::Regtest).unwrap();

    let target_address = client.get_new_address(None, None).unwrap();

    // make sure we have money
    client.generate(101, None).unwrap();

    let pattern = TransactionPattern {
        to_address: Some(target_address.clone()),
        from_outpoint: None,
        unlock_script: None,
    };

    let start_of_swap = Utc::now().naive_local();
    let funding_transaction = matching_transaction(connector, pattern, start_of_swap);
    let send_money_to_address = async {
        tokio::time::delay_for(Duration::from_secs(2)).await;
        tokio::task::spawn_blocking(move || {
            let transaction_hash = client
                .send_to_address(
                    &target_address,
                    Amount::from_sat(100_000_000),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap();
            client.generate(1, None).unwrap();

            transaction_hash
        })
        .await
    };

    let future = future::join(send_money_to_address, funding_transaction);

    let (actual_transaction, funding_transaction) =
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .unwrap();

    assert_eq!(
        funding_transaction.unwrap().txid(),
        actual_transaction.unwrap()
    )
}

pub fn new_bitcoincore_client<D: Docker>(
    container: &Container<'_, D, BitcoinCore>,
) -> bitcoincore_rpc::Client {
    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    bitcoincore_rpc::Client::new(
        endpoint,
        bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
    )
    .unwrap()
}
