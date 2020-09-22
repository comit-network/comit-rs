use crate::swap::{
    comit::{SwapFailedNoRefund, SwapFailedShouldRefund},
    hbit, herc20,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use comit::{
    btsieve,
    btsieve::{BlockByHash, LatestBlock},
    ethereum, Secret,
};

/// Execute a Hbit<->Herc20 swap for Alice.
#[allow(dead_code)] // This is library code
pub async fn hbit_herc20_alice<A, EC>(
    alice: A,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secret: Secret,
    utc_start_of_swap: DateTime<Utc>,
) -> anyhow::Result<()>
where
    A: hbit::ExecuteFund + herc20::ExecuteRedeem + hbit::ExecuteRefund,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    let swap_result = async {
        let hbit_funded = alice
            .execute_fund(&hbit_params)
            .await
            .context(SwapFailedNoRefund)?;

        let herc20_deployed = herc20::watch_for_deployed(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
        )
        .await
        .context(SwapFailedShouldRefund(hbit_funded))?;

        let _herc20_funded = herc20::watch_for_funded(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
            herc20_deployed.clone(),
        )
        .await
        .context(SwapFailedShouldRefund(hbit_funded))?;

        let _herc20_redeemed = alice
            .execute_redeem(herc20_params, secret, herc20_deployed, utc_start_of_swap)
            .await
            .context(SwapFailedShouldRefund(hbit_funded))?;

        Ok(())
    }
    .await;

    hbit::refund_if_necessary(alice, hbit_params, swap_result).await
}

/// Execute a Hbit<->Herc20 swap for Bob.
pub async fn hbit_herc20_bob<B, BC, EC>(
    bob: B,
    bitcoin_connector: &BC,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    utc_start_of_swap: DateTime<Utc>,
) -> Result<()>
where
    B: herc20::ExecuteDeploy + herc20::ExecuteFund + hbit::ExecuteRedeem + herc20::ExecuteRefund,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    tracing::info!("starting swap");

    let swap_result = async {
        let hbit_funded =
            hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, utc_start_of_swap)
                .await
                .context(SwapFailedNoRefund)?;

        tracing::info!("alice funded the hbit htlc");

        let herc20_deployed = bob
            .execute_deploy(herc20_params.clone())
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we deployed the herc20 htlc");

        let _herc20_funded = bob
            .execute_fund(
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we funded the herc20 htlc");

        let herc20_redeemed = herc20::watch_for_redeemed(
            ethereum_connector,
            utc_start_of_swap,
            herc20_deployed.clone(),
        )
        .await
        .context(SwapFailedShouldRefund(herc20_deployed.clone()))?;

        tracing::info!("alice redeemed the herc20 htlc");

        let _hbit_redeem = bob
            .execute_redeem(hbit_params, hbit_funded, herc20_redeemed.secret)
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we redeemed the hbit htlc");

        Ok(())
    }
    .await;

    herc20::refund_if_necessary(bob, herc20_params, utc_start_of_swap, swap_result).await
}
