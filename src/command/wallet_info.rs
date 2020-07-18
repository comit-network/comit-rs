use crate::{bitcoin, ethereum};

pub async fn wallet_info(
    ethereum_wallet: ethereum::Wallet,
    bitcoin_wallet: bitcoin::Wallet,
) -> anyhow::Result<String> {
    let bitcoin_info = bitcoin_info(bitcoin_wallet).await?;
    let ethereum_info = ethereum_info(ethereum_wallet);

    Ok(format!(
        "Bitcoin wallet descriptors:\n{}\nEthereum private key:\n{}",
        bitcoin_info, ethereum_info
    ))
}

async fn bitcoin_info(bitcoin_wallet: bitcoin::Wallet) -> anyhow::Result<String> {
    let descriptors = bitcoin_wallet.descriptors_with_checksums().await?;
    Ok(descriptors.join("\n"))
}

fn ethereum_info(ethereum_wallet: ethereum::Wallet) -> String {
    ethereum_wallet.private_key().to_string()
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{test_harness, Seed};

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn wallet_info_command() -> anyhow::Result<()> {
        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client)?;
        bitcoin_blockchain.init().await?;

        let bitcoin_wallet = bitcoin::Wallet::new(
            seed,
            bitcoin_blockchain.node_url,
            ::bitcoin::Network::Regtest,
        )
        .await?;

        let mut ethereum_blockchain = test_harness::ethereum::Blockchain::new(&client)?;
        ethereum_blockchain.init().await?;

        let ethereum_wallet = crate::ethereum::Wallet::new(
            seed,
            ethereum_blockchain.node_url.clone(),
            ethereum_blockchain.token_contract()?,
        )?;

        let stdout = wallet_info(ethereum_wallet, bitcoin_wallet).await?;
        println!("{}", stdout);
        Ok(())
    }
}
