use crate::{bitcoind, Seed};
use bitcoin::{Address, Amount, Network, Transaction, Txid};
use bitcoind::WalletInfoResponse;
use reqwest::Url;

#[derive(Clone, Debug)]
pub struct Wallet {
    name: String,
    bitcoind_client: bitcoind::Client,
    private_key: bitcoin::PrivateKey,
}

impl Wallet {
    pub fn new(seed: Seed, url: Url, network: Network, name: String) -> anyhow::Result<Wallet> {
        let private_key = bitcoin::PrivateKey {
            compressed: true,
            network,
            key: seed.secret_key()?,
        };

        let bitcoind_client = bitcoind::Client::new(url);

        Ok(Wallet {
            name,
            bitcoind_client,
            private_key,
        })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let info = self.info().await;

        // We assume the wallet present with the same name has the
        // same seed, which is fair but could be safer.
        if info.is_err() {
            // TODO: Probably need to protect the wallet with a passphrase
            self.bitcoind_client
                .create_wallet(&self.name, None, Some(true), None, None)
                .await?;

            let wif = self.private_key.to_wif();

            self.bitcoind_client
                .set_hd_seed(&self.name, Some(true), Some(wif))
                .await?;
        }

        Ok(())
    }

    pub async fn info(&self) -> anyhow::Result<WalletInfoResponse> {
        self.bitcoind_client.get_wallet_info(&self.name).await
    }

    pub async fn new_address(&self) -> anyhow::Result<Address> {
        self.bitcoind_client
            .get_new_address(&self.name, None, Some("bech32".into()))
            .await
    }

    pub async fn balance(&self) -> anyhow::Result<Amount> {
        self.bitcoind_client
            .get_balance(&self.name, None, None, None)
            .await
    }

    pub async fn send_to_address(
        &self,
        address: Address,
        amount: Amount,
        network: Network,
    ) -> anyhow::Result<Txid> {
        self.assert_network(network).await?;

        let txid = self
            .bitcoind_client
            .send_to_address(&self.name, address, amount)
            .await?;
        Ok(txid)
    }

    pub async fn send_raw_transaction(
        &self,
        transaction: Transaction,
        network: Network,
    ) -> anyhow::Result<Txid> {
        self.assert_network(network).await?;

        let txid = self
            .bitcoind_client
            .send_raw_transaction(&self.name, transaction)
            .await?;
        Ok(txid)
    }

    pub async fn get_raw_transaction(&self, txid: Txid) -> anyhow::Result<Transaction> {
        let transaction = self
            .bitcoind_client
            .get_raw_transaction(&self.name, txid)
            .await?;

        Ok(transaction)
    }

    async fn assert_network(&self, expected: Network) -> anyhow::Result<()> {
        let actual = self.bitcoind_client.network().await?;

        if expected != actual {
            anyhow::bail!("Wrong network: expected {}, got {}", expected, actual);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::new();
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod docker_tests {
    use super::*;
    use crate::test_harness::bitcoin;
    use testcontainers::clients;

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        let wallet = Wallet::new(
            seed,
            blockchain.node_url.clone(),
            Network::Regtest,
            "test".into(),
        )
        .unwrap();
        wallet.init().await.unwrap();

        let _address = wallet.new_address().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_balance() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        let wallet = Wallet::new(
            seed,
            blockchain.node_url.clone(),
            Network::Regtest,
            "test".into(),
        )
        .unwrap();
        wallet.init().await.unwrap();

        let _balance = wallet.balance().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_when_already_existing_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        {
            let wallet = Wallet::new(
                seed,
                blockchain.node_url.clone(),
                Network::Regtest,
                "test".into(),
            )
            .unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }

        {
            let wallet = Wallet::new(
                seed,
                blockchain.node_url.clone(),
                Network::Regtest,
                "test".into(),
            )
            .unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }
    }
}
