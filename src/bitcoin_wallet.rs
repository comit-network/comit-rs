use crate::bitcoind;
use ::bitcoin::hash_types::PubkeyHash;
use ::bitcoin::hashes::Hash;
use ::bitcoin::secp256k1;
use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use ::bitcoin::Address;
use ::bitcoin::Network;
use rand::prelude::*;
use reqwest::Url;

// TODO: Go in its own module
struct Seed([u8; SECRET_KEY_SIZE]);

impl Seed {
    pub fn new() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
    }

    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        Ok(secp256k1::SecretKey::from_slice(&self.0)?)
    }
}

struct Wallet {
    bitcoind_client: bitcoind::Client,
    private_key: ::bitcoin::PrivateKey,
}

impl Wallet {
    pub async fn new(seed: Seed, url: Url, network: Network) -> anyhow::Result<Wallet> {
        let private_key = ::bitcoin::PrivateKey {
            compressed: true,
            network,
            key: seed.secret_key()?,
        };
        let wif = private_key.to_wif();

        dbg!(&wif);

        let bitcoind_client = bitcoind::Client::new(url);

        let wallet = Wallet {
            bitcoind_client: bitcoind_client.clone(),
            private_key,
        };

        // TODO: Probably need to protect the wallet with a passphrase
        // TODO: Separate initialisation from constructor?
        bitcoind_client
            .create_wallet(&wallet.name(), None, Some(true), "".into(), None)
            .await?;

        bitcoind_client
            .set_hd_seed(&wallet.name(), Some(true), Some(wif))
            .await?;

        Ok(wallet)
    }

    pub async fn new_address(&self) -> anyhow::Result<Address> {
        self.bitcoind_client
            .get_new_address(&self.name(), None, Some("bech32".into()))
            .await
    }

    /// The wallet is named `nectar_x` with `x` being the first 4 byte of the public key hash
    //TODO: It's actually used often so better generate it in the constructor
    fn name(&self) -> String {
        let mut hash_engine = PubkeyHash::engine();
        self.private_key
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

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::new();
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

        let bitcoind_client = bitcoind::Client::new(blockchain.node_url.clone());

        // Create some test wallet
        let test_wallet_name = String::from("testwallet");
        bitcoind_client
            .create_wallet(&test_wallet_name, None, None, "".into(), None)
            .await
            .unwrap();
        // Get a test address
        let test_address = bitcoind_client
            .get_new_address(&test_wallet_name, None, None)
            .await
            .unwrap();
        // Generate blocks
        bitcoind_client
            .generate_to_address(101, test_address, None)
            .await
            .unwrap();

        let seed = Seed::new();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();

        let _address = wallet.new_address().await.unwrap();
    }
}
