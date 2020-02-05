use crate::{
    asset,
    asset::{Asset, Ether},
    btsieve::{ethereum, ethereum::Web3Connector},
    swap_protocols::{
        han::{Funded, Redeemed, Refunded, WatchFunded, WatchRedeemed, WatchRefunded},
        ledger,
        ledger::Ethereum,
        rfc003::create_swap::HtlcParams,
        Facade,
    },
};
use chrono::NaiveDateTime;

#[async_trait::async_trait]
impl HtlcFunded<bitcoin::Mainnet, asset::Bitcoin> for Facade {
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<bitcoin::Mainnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Mainnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<bitcoin::Mainnet, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<bitcoin::Mainnet, asset::Bitcoin> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<bitcoin::Mainnet, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<bitcoin::Mainnet>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<bitcoin::Mainnet, asset::Bitcoin> for Facade {
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<bitcoin::Mainnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Mainnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<bitcoin::Mainnet>> {
        self.bitcoin_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<bitcoin::Mainnet, asset::Bitcoin> for Facade {
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<bitcoin::Mainnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Mainnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<bitcoin::Mainnet>> {
        self.bitcoin_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcFunded<bitcoin::Testnet, asset::Bitcoin> for Facade {
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<bitcoin::Testnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Testnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<bitcoin::Testnet, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<bitcoin::Testnet, asset::Bitcoin> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<bitcoin::Testnet, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<bitcoin::Testnet>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<bitcoin::Testnet, asset::Bitcoin> for Facade {
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<bitcoin::Testnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Testnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<bitcoin::Testnet>> {
        self.bitcoin_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<bitcoin::Testnet, asset::Bitcoin> for Facade {
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<bitcoin::Testnet, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Testnet>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<bitcoin::Testnet>> {
        self.bitcoin_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcFunded<bitcoin::Regtest, asset::Bitcoin> for Facade {
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<bitcoin::Regtest, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Regtest>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<bitcoin::Regtest, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<bitcoin::Regtest, asset::Bitcoin> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<bitcoin::Regtest, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<bitcoin::Regtest>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<bitcoin::Regtest, asset::Bitcoin> for Facade {
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<bitcoin::Regtest, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Regtest>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<bitcoin::Regtest>> {
        self.bitcoin_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<bitcoin::Regtest, asset::Bitcoin> for Facade {
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<bitcoin::Regtest, asset::Bitcoin>,
        htlc_deployment: &Deployed<bitcoin::Regtest>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<bitcoin::Regtest>> {
        self.bitcoin_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl WatchFunded<Ethereum, Ether> for Facade
where
    ethereum::Cache<Web3Connector>: WatchFunded<Ethereum, Ether>,
{
    async fn watch_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, Ether>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Ethereum, Ether>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl WatchRedeemed<Ethereum, Ether> for Facade
where
    ethereum::Cache<Web3Connector>: WatchRedeemed<Ethereum, Ether>,
{
    async fn watch_redeemed(
        &self,
        htlc_params: HtlcParams<Ethereum, Ether>,
        funded: &Funded<Ethereum, Ether>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Ethereum>> {
        self.ethereum_connector
            .htlc_redeemed(htlc_params, funded, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl WatchRefunded<Ethereum, Ether> for Facade
where
    ethereum::Cache<Web3Connector>: WatchRefunded<Ethereum, Ether>,
{
    async fn watch_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, Ether>,
        funded: &Funded<Ethereum, Ether>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Ethereum>> {
        self.ethereum_connector
            .htlc_refunded(htlc_params, funded, start_of_swap)
            .await
    }
}
