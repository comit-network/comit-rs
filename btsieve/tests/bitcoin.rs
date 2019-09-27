use bitcoin_support::{Amount, Network};
use bitcoincore_rpc::RpcApi;
use btsieve::{
    bitcoin::{bitcoind_http_blocksource::BitcoindHttpBlockSource, TransactionQuery},
    matching_transactions::MatchingTransactions,
};
use futures::{Future, Stream};
use images::coblox_bitcoincore::BitcoinCore;
use reqwest::Url;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use testcontainers::*;
use tokio::{prelude::FutureExt, runtime::Runtime, timer::Delay};

/// A very basic e2e test that verifies that we glued all our code together
/// correctly for bitcoin queries
///
/// We send money to an address and check if the transaction that we filter out
/// is the same one as the one that was returned when we sent the money
#[test]
fn bitcoin_transaction_query_e2e_test() {
    let cli = clients::Cli::default();
    let container = cli.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);

    let mut url = Url::parse("http://localhost").unwrap();
    #[allow(clippy::cast_possible_truncation)]
    url.set_port(Some(container.get_host_port(18443).unwrap() as u16))
        .unwrap();

    let blocksource = Arc::new(BitcoindHttpBlockSource::new(url, Network::Regtest).unwrap());

    let target_address = client.get_new_address(None, None).unwrap();

    // make sure we have money
    client.generate(101, None).unwrap();

    let funding_transaction = blocksource
        .matching_transactions(TransactionQuery {
            to_address: Some(target_address.clone()),
            from_outpoint: None,
            unlock_script: None,
        })
        .take(1)
        .into_future()
        .map_err(|_| ());

    let now_in_two_seconds = Instant::now() + Duration::from_secs(2);

    let send_money_to_address = Delay::new(now_in_two_seconds)
        .map(move |_| {
            let transaction_hash = client
                .send_to_address(
                    &target_address,
                    Amount::from_btc(1.0).unwrap(),
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
        .map_err(|_| ());;

    let mut runtime = Runtime::new().unwrap();

    let future = send_money_to_address.join(funding_transaction);

    let future_with_timeout = future.timeout(Duration::from_secs(5));

    let (actual_transaction, (funding_transaction, _)) =
        runtime.block_on(future_with_timeout).unwrap();

    assert_eq!(funding_transaction.unwrap().txid(), actual_transaction)
}
