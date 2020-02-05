use crate::{
    asset::Erc20,
    btsieve::{ethereum, ethereum::Web3Connector},
    swap_protocols::{
        herc20::{Deployed, Funded, Redeemed, Refunded, WatchFunded, WatchRedeemed, WatchRefunded},
        ledger::Ethereum,
        rfc003::create_swap::HtlcParams,
        Facade,
    },
};
use chrono::NaiveDateTime;

#[async_trait::async_trait]
impl WatchFunded<Ethereum, Erc20> for Facade
where
    ethereum::Cache<Web3Connector>: WatchFunded<Ethereum, Erc20>,
{
    async fn watch_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Ethereum, Erc20>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl WatchRedeemed<Ethereum, Erc20> for Facade
where
    ethereum::Cache<Web3Connector>: WatchRedeemed<Ethereum, Erc20>,
{
    async fn watch_redeemed(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Ethereum>> {
        self.ethereum_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl WatchRefunded<Ethereum, Erc20> for Facade
where
    ethereum::Cache<Web3Connector>: WatchRefunded<Ethereum, Erc20>,
{
    async fn watch_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Ethereum>> {
        self.ethereum_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}
