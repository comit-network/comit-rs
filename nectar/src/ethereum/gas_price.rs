use crate::{
    config::EthereumGasPrice,
    ethereum::{ether, geth},
    Result,
};

mod eth_gas_station;

#[derive(Debug, Clone)]
pub struct GasPrice {
    service: Service,
}

#[derive(Debug, Clone)]
enum Service {
    Geth(geth::Client),
    EthGasStation(eth_gas_station::Client),
}

impl GasPrice {
    pub fn new(strategy: crate::config::EthereumGasPrice) -> Self {
        match strategy {
            EthereumGasPrice::Geth(url) => {
                let client = geth::Client::new(url);
                Self {
                    service: Service::Geth(client),
                }
            }
            EthereumGasPrice::EthGasStation(url) => {
                let client = eth_gas_station::Client::new(url);
                Self {
                    service: Service::EthGasStation(client),
                }
            }
        }
    }

    #[cfg(all(test, feature = "testcontainers"))]
    pub fn geth_url(geth_url: url::Url) -> Self {
        let client = geth::Client::new(geth_url);
        Self {
            service: Service::Geth(client),
        }
    }

    pub async fn gas_price(&self) -> Result<ether::Amount> {
        match &self.service {
            Service::Geth(client) => client.gas_price().await,
            Service::EthGasStation(client) => client.gas_price().await,
        }
    }
}

#[cfg(all(test, feature = "testcontainers"))]
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
