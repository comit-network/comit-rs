#[cfg(test)]
mod test {
    use bitcoin::{secp256k1, Network};
    use chrono::Utc;
    use comit::{
        asset, btsieve::bitcoin::BitcoindConnector, hbit, identity, Secret, SecretHash, Timestamp,
    };
    use futures::stream::TryStreamExt;
    use reqwest::Url;
    use std::str::FromStr;
    use testcontainers::{clients, images::coblox_bitcoincore::BitcoinCore, Container, Docker};

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
}
