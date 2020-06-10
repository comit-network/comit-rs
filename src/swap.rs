#[cfg(test)]
mod test {
    use super::*;
    use bitcoin::{secp256k1, Network};
    use chrono::Utc;
    use comit::{
        asset::{
            self,
            ethereum::{Erc20Quantity, FromWei},
        },
        btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
        ethereum, hbit, herc20, identity, Secret, SecretHash, Timestamp,
    };
    use futures::stream::TryStreamExt;
    use reqwest::Url;
    use std::str::FromStr;
    use testcontainers::{
        clients,
        images::coblox_bitcoincore::BitcoinCore,
        images::generic::{GenericImage, Stream, WaitFor},
        Container, Docker, Image,
    };

    struct BitcoinBlockchain<'c> {
        _container: Container<'c, clients::Cli, BitcoinCore>,
        node_url: Url,
    }

    impl<'c> BitcoinBlockchain<'c> {
        pub fn new(client: &'c clients::Cli) -> anyhow::Result<Self> {
            let container = client.run(BitcoinCore::default().with_tag("0.19.1"));
            let port = container.get_host_port(18443);

            let auth = container.image().auth();
            let url = format!(
                "http://{}:{}@localhost:{}",
                &auth.username,
                &auth.password,
                port.unwrap()
            );
            let url = Url::parse(&url)?;

            Ok(Self {
                _container: container,
                node_url: url,
            })
        }
    }

    struct EthereumBlockchain<'c> {
        _container: Container<'c, clients::Cli, GenericImage>,
        node_url: Url,
    }

    impl<'c> EthereumBlockchain<'c> {
        pub fn new(client: &'c clients::Cli) -> anyhow::Result<Self> {
            let geth_image = GenericImage::new("ethereum/client-go")
                .with_wait_for(WaitFor::LogMessage {
                    message: String::from("mined potential block"),
                    stream: Stream::StdErr,
                })
                .with_args(vec![
                    String::from("--dev"), // TODO: Most definitely missing arguments, see comit-rs geth_instance.ts
                    String::from("--dev.period=1"),
                    String::from("--rpc"),
                ]);

            let container = client.run(geth_image);
            let port = container.get_host_port(8545);

            let url = format!("http://localhost:{}", port.unwrap());
            let url = Url::parse(&url)?;

            Ok(Self {
                _container: container,
                node_url: url,
            })
        }
    }

    fn hbit_params(secret_hash: SecretHash) -> hbit::Params {
        // TODO: Build identities using transient secret keys
        let pk = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let identity = identity::Bitcoin::from(pk);

        hbit::Params {
            network: Network::Regtest,
            asset: asset::Bitcoin::from_sat(100_000_000),
            redeem_identity: identity,
            refund_identity: identity,
            expiry: Timestamp::from(0),
            secret_hash,
        }
    }

    fn herc20_params(secret_hash: SecretHash) -> herc20::Params {
        // TODO: Obtain from node configuration
        let token_contract =
            ethereum::Address::from_str("6b175474e89094c44da98b954eedeac495271d0f").unwrap();
        let quantity = Erc20Quantity::from_wei(1_000u32);
        let asset = asset::Erc20::new(token_contract, quantity);

        // TODO: Obtain identities from user wallets
        let identity =
            identity::Ethereum::from_str("c5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        herc20::Params {
            asset,
            redeem_identity: identity,
            refund_identity: identity,
            expiry: Timestamp::from(0),
            secret_hash,
        }
    }

    fn secret() -> Secret {
        let bytes = b"hello world, you are beautiful!!";
        Secret::from(*bytes)
    }

    #[tokio::test]
    async fn from_hbit_params_to_hbit_started() {
        let client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&client).unwrap();
        let connector = BitcoindConnector::new(blockchain.node_url, Network::Regtest).unwrap();

        let secret_hash = SecretHash::new(secret());
        let params = hbit_params(secret_hash);

        let mut events = hbit::new(&connector, params, Utc::now().naive_local());

        assert_eq!(events.try_next().await.unwrap(), Some(hbit::Event::Started));
    }

    #[tokio::test]
    async fn from_herc20_params_to_herc20_started() {
        let client = clients::Cli::default();
        let blockchain = EthereumBlockchain::new(&client).unwrap();
        let connector = Web3Connector::new(blockchain.node_url);

        let secret_hash = SecretHash::new(secret());
        let params = herc20_params(secret_hash);

        let mut events = herc20::new(&connector, params, Utc::now().naive_local());

        assert_eq!(
            events.try_next().await.unwrap(),
            Some(herc20::Event::Started)
        );
    }
}
