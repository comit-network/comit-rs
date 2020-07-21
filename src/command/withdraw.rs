use crate::command::Withdraw;
use crate::ethereum::STANDARD_ETH_TRANSFER_GAS_LIMIT;
use crate::{bitcoin, ethereum};

pub async fn withdraw(
    ethereum_wallet: ethereum::Wallet,
    bitcoin_wallet: bitcoin::Wallet,
    arguments: Withdraw,
) -> anyhow::Result<String> {
    match arguments {
        Withdraw::Btc { amount, to_address } => {
            let tx_id = bitcoin_wallet
                .send_to_address(to_address.clone(), amount, bitcoin_wallet.network)
                .await?;
            Ok(format!(
                "{} transferred to {}\nTransaction id: {}",
                amount, to_address, tx_id
            ))
        }
        Withdraw::Dai { .. } => todo!(),
        Withdraw::Eth { amount, to_address } => {
            let tx_id = ethereum_wallet
                .send_transaction(
                    to_address,
                    amount.clone(),
                    Some(STANDARD_ETH_TRANSFER_GAS_LIMIT),
                    None,
                    ethereum_wallet.chain_id,
                )
                .await?;
            Ok(format!(
                "{} transferred to {}\nTransaction id: {}",
                amount, to_address, tx_id
            ))
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::ethereum::{ether, ChainId};
    use crate::{test_harness, Seed};
    use std::str::FromStr;

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn withdraw_command() {
        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client).unwrap();
        bitcoin_blockchain.init().await.unwrap();

        let bitcoin_wallet = bitcoin::Wallet::new(
            seed,
            bitcoin_blockchain.node_url.clone(),
            ::bitcoin::Network::Regtest,
        )
        .await
        .unwrap();

        let bitcoin_address = bitcoin_wallet.new_address().await.unwrap();
        bitcoin_blockchain
            .mint(bitcoin_address, bitcoin::Amount::from_btc(1.2).unwrap())
            .await
            .unwrap();

        let mut ethereum_blockchain = test_harness::ethereum::Blockchain::new(&client).unwrap();
        ethereum_blockchain.init().await.unwrap();

        let ethereum_wallet = crate::ethereum::Wallet::new(
            seed,
            ethereum_blockchain.node_url.clone(),
            ethereum_blockchain.token_contract().unwrap(),
            ChainId::regtest(),
        )
        .await
        .unwrap();

        let ethereum_address = ethereum_wallet.account();
        ethereum_blockchain
            .mint_ether(
                ethereum_address,
                ether::Amount::from_ether_str("10").unwrap(),
                ChainId::regtest(),
            )
            .await
            .unwrap();

        let bitcoin_withdraw = Withdraw::Btc {
            amount: bitcoin::Amount::from_btc(0.3).unwrap(),
            to_address: bitcoin::Address::from_str("bcrt1qk60fmayw8xrtqd4ru2ut8kgv08wyqpdzqkj55h")
                .unwrap(),
        };
        let stdout = withdraw(
            ethereum_wallet.clone(),
            bitcoin_wallet.clone(),
            bitcoin_withdraw,
        )
        .await
        .unwrap();
        println!("{}", stdout);

        let ether_withdraw = Withdraw::Eth {
            amount: ether::Amount::from_ether_str("2.4").unwrap(),
            to_address: ethereum::Address::random(),
        };
        let stdout = withdraw(ethereum_wallet, bitcoin_wallet, ether_withdraw)
            .await
            .unwrap();
        println!("{}", stdout);
    }
}
