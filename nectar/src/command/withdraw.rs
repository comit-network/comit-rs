use crate::{bitcoin, command::Withdraw, ethereum, ethereum::STANDARD_ETH_TRANSFER_GAS_LIMIT};
use std::borrow::Borrow;

pub async fn withdraw(
    ethereum_wallet: ethereum::Wallet,
    bitcoin_wallet: impl Borrow<bitcoin::Wallet>,
    arguments: Withdraw,
) -> anyhow::Result<String> {
    match arguments {
        Withdraw::Btc { amount, to_address } => {
            let bitcoin_wallet = bitcoin_wallet.borrow();
            let tx_id = bitcoin_wallet
                .send_to_address(to_address.clone(), amount, bitcoin_wallet.network)
                .await?;
            Ok(format!(
                "{} transferred to {}\nTransaction id: {}",
                amount, to_address, tx_id
            ))
        }
        Withdraw::Dai { amount, to_address } => {
            let tx_id = ethereum_wallet
                .transfer_dai(to_address, amount.clone(), ethereum_wallet.chain_id())
                .await?;
            Ok(format!(
                "{} transferred to {}\nTransaction id: {}",
                amount, to_address, tx_id
            ))
        }
        Withdraw::Eth { amount, to_address } => {
            let tx_id = ethereum_wallet
                .send_transaction(
                    to_address,
                    amount.clone(),
                    Some(STANDARD_ETH_TRANSFER_GAS_LIMIT),
                    None,
                    ethereum_wallet.chain_id(),
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
    use crate::{
        ethereum::{dai, ether, ChainId},
        test_harness, Seed,
    };
    use comit::asset::{ethereum::FromWei, Erc20, Erc20Quantity};
    use std::{str::FromStr, sync::Arc};

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn withdraw_command() {
        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client).unwrap();
        bitcoin_blockchain.init().await.unwrap();

        let bitcoin_wallet = Arc::new(
            bitcoin::Wallet::new(
                seed,
                bitcoin_blockchain.node_url.clone(),
                ::bitcoin::Network::Regtest,
            )
            .await
            .unwrap(),
        );

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
            ethereum::Chain::new(ChainId::GETH_DEV, ethereum_blockchain.token_contract()),
        )
        .await
        .unwrap();

        let ethereum_address = ethereum_wallet.account();
        ethereum_blockchain
            .mint_ether(
                ethereum_address,
                ether::Amount::from_ether_str("10").unwrap(),
                ChainId::GETH_DEV,
            )
            .await
            .unwrap();
        ethereum_blockchain
            .mint_erc20_token(
                ethereum_address,
                Erc20 {
                    quantity: Erc20Quantity::from_wei(10_000_000_000_000_000_000u64),
                    token_contract: ethereum_blockchain.token_contract(),
                },
                ChainId::GETH_DEV,
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
        let stdout = withdraw(
            ethereum_wallet.clone(),
            bitcoin_wallet.clone(),
            ether_withdraw,
        )
        .await
        .unwrap();
        println!("{}", stdout);

        let dai_withdraw = Withdraw::Dai {
            amount: dai::Amount::from_dai_trunc(3.2).unwrap(),
            to_address: ethereum::Address::random(),
        };
        let stdout = withdraw(ethereum_wallet, bitcoin_wallet, dai_withdraw)
            .await
            .unwrap();
        println!("{}", stdout);
    }
}
