use crate::{
    bitcoin::{Address, Amount, Client, Network, WalletInfoResponse},
    seed::Seed,
};
use ::bitcoin::{
    hashes::{sha256, Hash, HashEngine},
    secp256k1::SecretKey,
    util::bip32::{ChainCode, ChildNumber, ExtendedPrivKey},
    PrivateKey, Transaction, Txid,
};
use bitcoin::{util::bip32::DerivationPath, OutPoint};
use std::str::FromStr;
use url::Url;

const BITCOIND_DEFAULT_EXTERNAL_DERIVATION_PATH: &str = "/0h/0h/*h";
const BITCOIND_DEFAULT_INTERNAL_DERIVATION_PATH: &str = "/0h/1h/*h";
const TRANSIENT_DERIVATION_PATH: &str = "m/0'/9939'";

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Wallet {
    /// The wallet is named `nectar_x` with `x` being the first 4 bytes of the
    /// hash of the seed
    name: String,
    bitcoind_client: Client,
    root_key: ExtendedPrivKey,
    pub network: Network,
}

impl Wallet {
    pub async fn new(seed: Seed, url: Url, network: Network) -> anyhow::Result<Wallet> {
        let name = Wallet::gen_name(seed);
        let bitcoind_client = Client::new(url);

        let root_key = Self::root_extended_private_key_from_seed(&seed, network);

        let wallet = Wallet {
            name,
            bitcoind_client,
            root_key,
            network,
        };

        wallet.init(seed).await?;

        Ok(wallet)
    }

    async fn init(&self, seed: Seed) -> anyhow::Result<()> {
        let info = self.info().await;

        // We assume the wallet present with the same name has the
        // same seed, which is fair but could be safer.
        match info {
            Err(_) => {
                // TODO: Probably need to protect the wallet with a passphrase
                self.bitcoind_client
                    .create_wallet(&self.name, None, Some(true), None, None)
                    .await?;

                let wif = self.seed_as_wif(seed);

                self.bitcoind_client
                    .set_hd_seed(&self.name, Some(true), Some(wif))
                    .await?;

                self.bitcoind_client.rescan(&self.name).await?;

                Ok(())
            }
            Ok(WalletInfoResponse {
                hd_seed_id: None, ..
            }) => {
                // The wallet may have been previously created, but the `sethdseed` call may
                // have failed
                let wif = self.seed_as_wif(seed);

                self.bitcoind_client
                    .set_hd_seed(&self.name, Some(true), Some(wif))
                    .await?;

                // Rescan wallet to ensure funding is picked up
                self.bitcoind_client.rescan(&self.name).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Derive a new key under transient derivation path
    pub fn derive_transient_sk(&self, index: u32) -> anyhow::Result<SecretKey> {
        let index = ChildNumber::from_hardened_idx(index)?;
        let path = DerivationPath::from_str(TRANSIENT_DERIVATION_PATH)
            .expect("Valid derivation path in cost")
            .child(index);
        let ext_key = self.root_key.derive_priv(&crate::SECP, &path)?;
        Ok(ext_key.private_key.key)
    }

    pub async fn info(&self) -> anyhow::Result<WalletInfoResponse> {
        self.assert_network(self.network).await?;

        self.bitcoind_client.get_wallet_info(&self.name).await
    }

    pub async fn new_address(&self) -> anyhow::Result<Address> {
        self.assert_network(self.network).await?;

        self.bitcoind_client
            .get_new_address(&self.name, None, Some("bech32".into()))
            .await
    }

    pub async fn balance(&self) -> anyhow::Result<Amount> {
        self.assert_network(self.network).await?;

        self.bitcoind_client
            .get_balance(&self.name, None, None, None)
            .await
    }

    /// Returns the seed in wif format, this allows the user to import the
    /// wallet in a different bitcoind using `sethdseed`.
    /// It seems relevant that access to bitcoind must not be needed to complete
    /// the task in case there is an issue with bitcoind and the user wants
    /// to regain control over their wallet Do note that the `wif` format is
    /// only here to allow the communication of `bytes`. The seed
    /// is NOT used as a private key in bitcoin. See `root_extended_private_key`
    /// to get the root private key of the bip32 hd wallet.
    // TODO: check the network against bitcoind in a non-failing manner (just log)
    pub fn seed_as_wif(&self, seed: Seed) -> String {
        let key = seed.as_secret_key();

        let private_key = PrivateKey {
            compressed: true,
            network: self.network,
            key,
        };

        private_key.to_wif()
    }

    pub fn root_extended_private_key_from_seed(seed: &Seed, network: Network) -> ExtendedPrivKey {
        let (key, chain_code) = seed.root_secret_key_chain_code();
        let chain_code = ChainCode::from(chain_code.as_slice());

        let private_key = PrivateKey {
            compressed: true,
            network,
            key,
        };

        ExtendedPrivKey {
            network,
            depth: 0,
            parent_fingerprint: Default::default(),
            child_number: 0.into(),
            private_key,
            chain_code,
        }
    }

    /// Wallet descriptors as specified in https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md
    pub fn descriptors(&self) -> Vec<String> {
        Self::hd_paths()
            .iter()
            .map(|path| format!("wpkh({}{})", self.root_key, path))
            .collect()
    }

    pub fn descriptors_from_seed(seed: &Seed, network: Network) -> Vec<String> {
        let ext_priv_key = Self::root_extended_private_key_from_seed(seed, network);
        Self::hd_paths()
            .iter()
            .map(|path| format!("wpkh({}{})", ext_priv_key, path))
            .collect()
    }

    /// Some bitcoind rpc command requires the descriptor to be
    /// suffixed with a checksum. For now we are going to ask bitcoind
    /// to calculate the checksum for us.
    pub async fn descriptors_with_checksums(&self) -> anyhow::Result<Vec<String>> {
        let mut descriptors = Vec::new();
        for descriptor in self.descriptors() {
            let response = self
                .bitcoind_client
                .get_descriptor_info(&descriptor)
                .await?;
            let descriptor = format!("{}#{}", descriptor, response.checksum);
            descriptors.push(descriptor);
        }

        Ok(descriptors)
    }

    /// In accordance with [BIP32](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki),
    /// bitcoind uses 2 derivations paths to generate new keys and addresses,
    /// "m/iH/0/k corresponds to the k'th keypair of the external chain of
    /// account number i of the HDW derived from master m." ie, the
    /// addresses to give to someone else to receive bitcoin.
    /// "m/iH/1/k corresponds to the k'th keypair of the internal chain of
    /// account number i of the HDW derived from master m." ie, the
    /// addresses to send change.
    fn hd_paths() -> Vec<&'static str> {
        vec![
            BITCOIND_DEFAULT_EXTERNAL_DERIVATION_PATH,
            BITCOIND_DEFAULT_INTERNAL_DERIVATION_PATH,
        ]
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

    pub async fn fund_htlc(
        &self,
        address: Address,
        amount: Amount,
        network: Network,
    ) -> anyhow::Result<OutPoint> {
        self.assert_network(network).await?;

        let outpoint = self
            .bitcoind_client
            .fund_htlc(&self.name, address, amount)
            .await?;
        Ok(outpoint)
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

    #[cfg(test)]
    pub async fn dump(&self, filename: &std::path::Path) -> anyhow::Result<()> {
        self.bitcoind_client.dump_wallet(&self.name, filename).await
    }

    async fn assert_network(&self, expected: Network) -> anyhow::Result<()> {
        let actual = self.bitcoind_client.network().await?;

        if expected != actual {
            anyhow::bail!("Wrong network: expected {}, got {}", expected, actual);
        }

        Ok(())
    }

    fn gen_name(seed: Seed) -> String {
        let mut engine = sha256::HashEngine::default();

        engine.input(&seed.bytes());

        let hash = sha256::Hash::from_engine(engine);
        let hash = hash.into_inner();

        format!(
            "nectar_{:x}{:x}{:x}{:x}",
            hash[0], hash[1], hash[2], hash[3]
        )
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod docker_tests {
    use super::*;
    use crate::test_harness::bitcoin;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        process::{Command, Stdio},
    };
    use tempfile::TempDir;
    use testcontainers::clients;

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::random().unwrap();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();

        let _address = wallet.new_address().await.unwrap();
    }

    #[tokio::test]
    async fn root_key_calculated_from_seed_is_the_same_than_bitcoind_s() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::random().unwrap();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();

        let wif_path_docker = Path::new("/wallet.wif");

        let _ = wallet.dump(wif_path_docker).await.unwrap();

        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join("wallet.wif");

        Command::new("docker")
            .arg("cp")
            .arg(format!(
                "{}:{}",
                blockchain.container_id(),
                wif_path_docker.display()
            ))
            .arg(path.clone())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker cp")
            .wait()
            .expect("Failed to run docker cp");

        let wif = File::open(path).unwrap();
        let wif = BufReader::new(wif);

        // The line we are looking for looks like that:
        // # extended private masterkey: tprv...

        let line = wif
            .lines()
            .find(|line| {
                line.as_ref()
                    .map(|line| line.starts_with("# extended private masterkey: "))
                    .unwrap_or(false)
            })
            .unwrap()
            .unwrap();

        let key = line.split_ascii_whitespace().last().unwrap();
        assert_eq!(key, &wallet.root_key.to_string());
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_balance() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::random().unwrap();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();

        let _balance = wallet.balance().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_when_already_existing_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::random().unwrap();
        {
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
                .await
                .unwrap();

            let _address = wallet.new_address().await.unwrap();
        }

        {
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
                .await
                .unwrap();

            let _address = wallet.new_address().await.unwrap();
        }
    }

    // The test does not behave the same way than I encountered when running a solo
    // container Let's not invest more time on it right now and review later.
    #[ignore]
    #[tokio::test]
    async fn create_bitcoin_wallet_when_already_existing_but_no_seed_set_and_get_address() {
        let tc_client = clients::Cli::default();
        let seed = Seed::random().unwrap();

        // Get the wallet name for the seed
        let wallet_name = {
            let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();
            blockchain.init().await.unwrap();
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
                .await
                .unwrap();
            wallet.name
        };

        // The trick is to not generate 100 blocks, bitcoind will accept bitcoin_wallet
        // creation but fail setting the seed (but for some reason I am not able
        // to reproduce this behaviour)
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();
        {
            let res = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).await;
            // If this did not fail then the test is moot
            assert!(res.is_err());

            let list_wallets = Client::new(blockchain.node_url.clone())
                .list_wallets()
                .await
                .unwrap();
            // If the wallet is not created the test is moot
            assert!(list_wallets.contains(&wallet_name));
        }
        // Generate 100+ blocks, now it should work
        blockchain.init().await.unwrap();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();
        let _address = wallet.new_address().await.unwrap();
        // If we did not panic, we succeeded.
    }

    #[tokio::test]
    async fn descriptor_generates_same_addresses_than_bitcoin_wallet() {
        let seed = Seed::random().unwrap();

        let addresses = {
            let tc_client = clients::Cli::default();
            let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();
            blockchain.init().await.unwrap();

            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
                .await
                .unwrap();

            let mut addresses = Vec::new();

            // This is not ideal because it only returns the "external" addresses
            for _ in 0u8..20 {
                addresses.push(wallet.new_address().await.unwrap())
            }
            addresses
        };

        assert_ne!(addresses.len(), 0);

        // Start a new node just to be sure there is no mix up
        let tc_client = clients::Cli::default();
        let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();
        blockchain.init().await.unwrap();
        let bitcoind_client = Client::new(blockchain.node_url.clone());
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest)
            .await
            .unwrap();

        let descriptors = wallet.descriptors_with_checksums().await.unwrap();

        // This returns 40 addresses, 20 per path but the "internal" path used for
        // change Addresses will not be tested.
        let mut derived_addresses = Vec::new();
        for descriptor in descriptors {
            let mut addresses = bitcoind_client
                .derive_addresses(descriptor.as_str(), Some([0, 20]))
                .await
                .unwrap();
            derived_addresses.append(&mut addresses);
        }

        for address in addresses {
            assert!(derived_addresses.contains(&address))
        }
    }
}
