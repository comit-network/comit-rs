use crate::swap::{
    comit::{SwapFailedNoRefund, SwapFailedShouldRefund},
    hbit, herc20,
};
use anyhow::Context;
use chrono::{DateTime, Utc};
use comit::{
    btsieve,
    btsieve::{BlockByHash, LatestBlock},
    ethereum, Secret,
};

/// Execute a Herc20<->Hbit swap for Alice.
#[allow(dead_code)] // This is library code
pub async fn herc20_hbit_alice<A, BC>(
    alice: A,
    bitcoin_connector: &BC,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    secret: Secret,
    utc_start_of_swap: DateTime<Utc>,
) -> anyhow::Result<()>
where
    A: herc20::ExecuteDeploy + herc20::ExecuteFund + herc20::ExecuteRefund + hbit::ExecuteRedeem,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
{
    let swap_result = async {
        let herc20_deployed = alice
            .execute_deploy(herc20_params.clone())
            .await
            .context(SwapFailedNoRefund)?;

        let _herc20_funded = alice
            .execute_fund(
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedNoRefund)?;

        let hbit_funded =
            hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, utc_start_of_swap)
                .await
                .context(SwapFailedShouldRefund(herc20_deployed.clone()))?;

        let _hbit_redeemed = alice
            .execute_redeem(hbit_params, hbit_funded, secret)
            .await
            .context(SwapFailedShouldRefund(herc20_deployed))?;

        Ok(())
    }
    .await;

    herc20::refund_if_necessary(alice, herc20_params, utc_start_of_swap, swap_result).await
}

/// Execute a Herc20<->Hbit swap for Bob.
pub async fn herc20_hbit_bob<B, EC, BC>(
    bob: B,
    ethereum_connector: &EC,
    bitcoin_connector: &BC,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: DateTime<Utc>,
) -> anyhow::Result<()>
where
    B: hbit::ExecuteFund + hbit::ExecuteRefund + herc20::ExecuteRedeem,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
{
    tracing::info!("starting swap");

    let swap_result = async {
        let herc20_deployed = herc20::watch_for_deployed(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
        )
        .await
        .context(SwapFailedNoRefund)?;

        tracing::info!("alice deployed the herc20 htlc");

        let _herc20_funded = herc20::watch_for_funded(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
            herc20_deployed.clone(),
        )
        .await
        .context(SwapFailedNoRefund)?;

        tracing::info!("alice funded the herc20 htlc");

        let hbit_funded = bob
            .execute_fund(&hbit_params)
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we funded the hbit htlc");

        let hbit_redeemed = hbit::watch_for_redeemed(
            bitcoin_connector,
            &hbit_params.shared,
            hbit_funded.location,
            utc_start_of_swap,
        )
        .await
        .context(SwapFailedShouldRefund(hbit_funded))?;

        tracing::info!("alice redeemed the hbit htlc");

        let _herc20_redeem = bob
            .execute_redeem(
                herc20_params,
                hbit_redeemed.secret,
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we redeemed the herc20 htlc");

        Ok(())
    }
    .await;

    hbit::refund_if_necessary(bob, hbit_params, swap_result).await
}
