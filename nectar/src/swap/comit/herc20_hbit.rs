use crate::swap::{
    comit::{SwapFailedNoRefund, SwapFailedShouldRefund},
    hbit, herc20,
};
use anyhow::Context;
use time::OffsetDateTime;

/// Execute a Herc20<->Hbit swap for Bob.
pub async fn herc20_hbit_bob<B>(
    bob: B,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: OffsetDateTime,
) -> anyhow::Result<()>
where
    B: hbit::ExecuteFund
        + hbit::ExecuteRefund
        + herc20::ExecuteRedeem
        + herc20::WatchForDeployed
        + herc20::WatchForFunded
        + hbit::WatchForRedeemed,
{
    tracing::info!("starting swap");

    let swap_result = async {
        let herc20_deployed = bob
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("alice deployed the herc20 htlc");

        let _herc20_funded = bob
            .watch_for_funded(
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("alice funded the herc20 htlc");

        let hbit_funded = bob
            .execute_fund(&hbit_params)
            .await
            .context(SwapFailedNoRefund)?;

        tracing::info!("we funded the hbit htlc");

        let hbit_redeemed = bob
            .watch_for_redeemed(&hbit_params.shared, hbit_funded, utc_start_of_swap)
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
