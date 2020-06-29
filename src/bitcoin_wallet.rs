use crate::bitcoind;
use crate::bitcoind::WalletInfoResponse;
use crate::seed::Seed;
use ::bitcoin::hash_types::PubkeyHash;
use ::bitcoin::hashes::Hash;
use ::bitcoin::Address;
use ::bitcoin::Network;
use bitcoin::{Amount, PrivateKey};
use reqwest::Url;

struct Wallet {
    /// The wallet is named `nectar_x` with `x` being the first 4 byte of the public key hash
    name: String,
    bitcoind_client: bitcoind::Client,
    private_key: ::bitcoin::PrivateKey,
}

impl Wallet {
    pub fn new(seed: Seed, url: Url, network: Network) -> anyhow::Result<Wallet> {
        let key = seed.secret_key()?;

        let private_key = ::bitcoin::PrivateKey {
            compressed: true,
            network,
            key,
        };

        let bitcoind_client = bitcoind::Client::new(url);

        let name = Wallet::gen_name(private_key);

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
                .create_wallet(&self.name, None, Some(true), "".into(), None)
                .await?;

            let wif = self.wif();

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

    /// Returns the private key in wif format, this allows the user to import the wallet in a
    /// different bitcoind using `sethdseed`.
    /// It seems relevant that access to bitcoind must not be needed to complete the task
    /// in case there is an issue with bitcoind and the user wants to regain control over their wallet
    pub fn wif(&self) -> String {
        self.private_key.to_wif()
    }

    fn gen_name(private_key: PrivateKey) -> String {
        let mut hash_engine = PubkeyHash::engine();
        private_key
            .public_key(&crate::SECP)
            .write_into(&mut hash_engine);

        let public_key_hash = PubkeyHash::from_engine(hash_engine);

        format!(
            "nectar_{:x}{:x}{:x}{:x}",
            public_key_hash[0], public_key_hash[1], public_key_hash[2], public_key_hash[3]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn create_new_wallet() {
        let seed = Seed::default();
        let url = Url::from_str("http://localhost:1234").unwrap();

        let res = Wallet::new(seed, url, Network::Regtest);
        assert!(res.is_ok())
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod docker_tests {
    use super::*;
    use crate::test_harness::BitcoinBlockchain;
    use testcontainers::clients;

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::default();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
        wallet.init().await.unwrap();

        let _address = wallet.new_address().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_balance() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::default();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
        wallet.init().await.unwrap();

        let _balance = wallet.balance().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_when_already_existing_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::default();
        {
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }

        {
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }
    }
}
