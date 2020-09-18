use crate::{bitcoin, ethereum};

pub async fn balance(
    ethereum_wallet: ethereum::Wallet,
    bitcoin_wallet: bitcoin::Wallet,
) -> anyhow::Result<String> {
    let bitcoin_balance = bitcoin_wallet
        .balance()
        .await
        .map(|amount| amount.to_string())
        .unwrap_or_else(|e| format!("Problem encountered: {:#}", e));
    let dai_balance = ethereum_wallet
        .dai_balance()
        .await
        .map(|amount| amount.to_string())
        .unwrap_or_else(|e| format!("Problem encountered: {:#}", e));
    let ether_balance = ethereum_wallet
        .ether_balance()
        .await
        .map(|amount| amount.to_string())
        .unwrap_or_else(|e| format!("Problem encountered: {:#}", e));

    Ok(format!(
        "Bitcoin: {}\nDai: {}\nEther: {}",
        bitcoin_balance, dai_balance, ether_balance
    ))
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{ethereum, test_harness, Seed};
    use comit::ethereum::ChainId;

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn balance_command() {
        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client).unwrap();
        bitcoin_blockchain.init().await.unwrap();

        let bitcoin_wallet = bitcoin::Wallet::new(
            seed,
            bitcoin_blockchain.node_url,
            ::bitcoin::Network::Regtest,
        )
        .await
        .unwrap();

        let mut ethereum_blockchain = test_harness::ethereum::Blockchain::new(&client).unwrap();
        ethereum_blockchain.init().await.unwrap();

        let ethereum_wallet = crate::ethereum::Wallet::new(
            seed,
            ethereum_blockchain.node_url.clone(),
            ethereum::Chain::new(ChainId::GETH_DEV, ethereum_blockchain.token_contract()),
        )
        .await
        .unwrap();

        let stdout = balance(ethereum_wallet, bitcoin_wallet).await.unwrap();
        println!("{}", stdout);
    }
}
