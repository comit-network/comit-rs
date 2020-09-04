use crate::{bitcoin, ethereum, Seed};

pub async fn wallet_info(
    ethereum_wallet: Option<ethereum::Wallet>,
    bitcoin_wallet: Option<bitcoin::Wallet>,
    seed: &Seed,
    bitcoin_network: bitcoin::Network,
) -> anyhow::Result<String> {
    let bitcoin_info = bitcoin_info(bitcoin_wallet, &seed, bitcoin_network).await;
    let ethereum_info = ethereum_info(ethereum_wallet, &seed);

    Ok(format!(
        "Bitcoin wallet descriptors:\n{}\nEthereum private key:\n{}",
        bitcoin_info, ethereum_info
    ))
}

async fn bitcoin_info(
    bitcoin_wallet: Option<bitcoin::Wallet>,
    seed: &Seed,
    network: bitcoin::Network,
) -> String {
    let descriptors = match bitcoin_wallet {
        Some(bitcoin_wallet) => bitcoin_wallet.descriptors_with_checksums().await.ok(),
        None => None,
    };

    match descriptors {
        Some(descriptors) => descriptors.join("\n"),
        None => {
            let descriptors = bitcoin::Wallet::descriptors_from_seed(&seed, network);
            format!("(could not reach bitcoind)\n{}", descriptors.join("\n"))
        }
    }
}

fn ethereum_info(ethereum_wallet: Option<ethereum::Wallet>, seed: &Seed) -> String {
    match ethereum_wallet {
        Some(ethereum_wallet) => ethereum_wallet.private_key().to_string(),
        None => ethereum::Wallet::private_key_from_seed(seed)
            .expect("Derive private key from seed")
            .to_string(),
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{test_harness, Seed};
    use comit::ethereum::ChainId;

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
            ethereum::Chain::new(ChainId::GETH_DEV, ethereum_blockchain.token_contract()),
        )
        .await?;

        let stdout = wallet_info(
            Some(ethereum_wallet),
            Some(bitcoin_wallet),
            &seed,
            bitcoin::Network::Regtest,
        )
        .await?;
        println!("{}", stdout);
        Ok(())
    }

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn wallet_info_command_no_nodes() -> anyhow::Result<()> {
        let seed = Seed::random().unwrap();

        let stdout = wallet_info(None, None, &seed, bitcoin::Network::Regtest).await?;
        println!("{}", stdout);
        Ok(())
    }
}
