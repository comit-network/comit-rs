use crate::bitcoind;
use reqwest::Url;
use testcontainers::{
    clients,
    images::coblox_bitcoincore::BitcoinCore,
    images::generic::{GenericImage, Stream, WaitFor},
    Container, Docker, Image,
};

#[derive(Debug)]
pub struct BitcoinBlockchain<'c> {
    _container: Container<'c, clients::Cli, BitcoinCore>,
    pub node_url: Url,
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

    pub async fn init(&self) -> anyhow::Result<()> {
        let bitcoind_client = bitcoind::Client::new(self.node_url.clone());

        let test_wallet_name = String::from("testwallet");
        bitcoind_client
            .create_wallet(&test_wallet_name, None, None, "".into(), None)
            .await?;

        let test_address = bitcoind_client
            .get_new_address(&test_wallet_name, None, None)
            .await?;

        bitcoind_client
            .generate_to_address(101, test_address, None)
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct EthereumBlockchain<'c> {
    _container: Container<'c, clients::Cli, GenericImage>,
    pub node_url: Url,
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
