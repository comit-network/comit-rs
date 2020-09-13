use bitcoincore_rpc::RpcApi;
use chrono::offset::Utc;
use comit::btsieve::bitcoin::{watch_for_created_outpoint, BitcoindConnector};
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

    let connector = BitcoindConnector::new(url).unwrap();

    let target_address = client.get_new_address(None, None).unwrap();
    // make sure we have money
    client.generate_to_address(101, &target_address).unwrap();

    // A random address to send further bitcoin to when generating
    let dummy_address = "bcrt1qnylgnvd94ukm43e4dh9vxefywhlr9zdgz2g86f"
        .parse()
        .unwrap();

    let start_of_swap = Utc::now();

    let send_money_to_address = async {
        tokio::time::delay_for(Duration::from_secs(2)).await;
        tokio::task::spawn_blocking({
            let target_address = target_address.clone();
            move || {
                let transaction_hash = client
                    .send_to_address(
                        &target_address,
                        bitcoincore_rpc::bitcoin::Amount::from_sat(100_000_000),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                client.generate_to_address(1, &dummy_address).unwrap();

                transaction_hash
            }
        })
        .await
    };

    let actual_transaction = tokio::time::timeout(Duration::from_secs(5), send_money_to_address)
        .await
        .expect("failed to send money to address");

    let (funding_transaction, _out_point) = watch_for_created_outpoint(
        &connector,
        start_of_swap,
        target_address.to_string().parse().unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        funding_transaction.txid().to_string(),
        actual_transaction.unwrap().to_string()
    )
}

pub fn new_bitcoincore_client<D>(
    container: &Container<'_, D, BitcoinCore>,
) -> bitcoincore_rpc::Client
where
    D: Docker,
{
    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    bitcoincore_rpc::Client::new(
        endpoint,
        bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
    )
    .unwrap()
}
