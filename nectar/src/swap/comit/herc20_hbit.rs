use crate::swap::{
    comit::{SwapFailedNoRefund, SwapFailedShouldRefund},
    hbit, herc20,
};
use anyhow::Context;
use comit::{
    btsieve,
    btsieve::{BlockByHash, ConnectedNetwork, LatestBlock},
    ethereum,
    ethereum::ChainId,
    ledger,
};
use time::OffsetDateTime;

/// Execute a Herc20<->Hbit swap for Bob.
pub async fn herc20_hbit_bob<B, EC, BC>(
    bob: B,
    ethereum_connector: &EC,
    bitcoin_connector: &BC,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: OffsetDateTime,
) -> anyhow::Result<()>
where
    B: hbit::ExecuteFund + hbit::ExecuteRefund + herc20::ExecuteRedeem + herc20::WatchForDeployed,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash
        + ConnectedNetwork<Network = ChainId>,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    tracing::info!("starting swap");

    let swap_result = async {
        let herc20_deployed = bob
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
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
