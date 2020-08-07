use crate::bitcoin;
use std::time::Duration;
use testcontainers::{clients, images::coblox_bitcoincore::BitcoinCore, Container, Docker};
use url::Url;

#[derive(Debug)]
pub struct Blockchain<'c> {
    _container: Container<'c, clients::Cli, BitcoinCore>,
    pub node_url: Url,
    pub wallet_name: String,
}

impl<'c> Blockchain<'c> {
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

        let wallet_name = String::from("testwallet");

        Ok(Self {
            _container: container,
            node_url: url,
            wallet_name,
        })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let bitcoind_client = bitcoin::Client::new(self.node_url.clone());

        bitcoind_client
            .create_wallet(&self.wallet_name, None, None, None, None)
            .await?;

        let reward_address = bitcoind_client
            .get_new_address(&self.wallet_name, None, None)
            .await?;

        bitcoind_client
            .generate_to_address(105, reward_address.clone(), None)
            .await?;
        let _ = tokio::spawn(mine(bitcoind_client, reward_address));

        Ok(())
    }

    pub async fn mint(
        &self,
        address: bitcoin::Address,
        amount: bitcoin::Amount,
    ) -> anyhow::Result<()> {
        let bitcoind_client = bitcoin::Client::new(self.node_url.clone());

        bitcoind_client
            .send_to_address(&self.wallet_name, address.clone(), amount)
            .await?;

        Ok(())
    }

    pub fn container_id(&self) -> &str {
        self._container.id()
    }
}

async fn mine(
    bitcoind_client: bitcoin::Client,
    reward_address: bitcoin::Address,
) -> anyhow::Result<()> {
    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        bitcoind_client
            .generate_to_address(1, reward_address.clone(), None)
            .await?;
    }
}
