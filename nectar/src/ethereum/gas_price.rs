use crate::{
    ethereum::{ether, geth},
    Result,
};

#[derive(Debug, Clone)]
pub struct GasPrice {
    service: Service,
}

#[derive(Debug, Clone)]
enum Service {
    Geth(geth::Client),
}

impl GasPrice {
    pub fn geth_url(geth_url: url::Url) -> Self {
        let client = geth::Client::new(geth_url);
        Self {
            service: Service::Geth(client),
        }
    }

    pub async fn gas_price(&self) -> Result<ether::Amount> {
        match &self.service {
            Service::Geth(client) => client.gas_price().await,
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::test_harness::ethereum::Blockchain;

    #[tokio::test]
    async fn gas_price() {
        let client = testcontainers::clients::Cli::default();

        let mut blockchain = Blockchain::new(&client).unwrap();
        blockchain.init().await.unwrap();

        let gas_price = GasPrice::geth_url(blockchain.node_url.clone());

        let gas_price = gas_price.gas_price().await.unwrap();

        println!("Gas price: {}", gas_price)
    }
}
