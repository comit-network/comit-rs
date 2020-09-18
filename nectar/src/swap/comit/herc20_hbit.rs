use crate::swap::{hbit, herc20};
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
    use Herc20HbitAliceError::*;

    let happy_path = async {
        let herc20_deployed = alice
            .execute_deploy(herc20_params.clone())
            .await
            .map_err(|_| AliceDeploy)?;

        let _herc20_funded = alice
            .execute_fund(
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .map_err(|_| AliceFund)?;

        let hbit_funded =
            hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, utc_start_of_swap)
                .await
                .map_err(|_| BobFund(herc20_deployed.clone()))?;

        let _hbit_redeemed = alice
            .execute_redeem(hbit_params, hbit_funded, secret)
            .await
            .map_err(|_| AliceRedeem(herc20_deployed))?;

        Ok(())
    };

    if let Err(BobFund(herc20_deployed)) | Err(AliceRedeem(herc20_deployed)) = happy_path.await {
        alice
            .execute_refund(herc20_params, herc20_deployed, utc_start_of_swap)
            .await?;
    };

    Ok(())
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
    use Herc20HbitBobError::*;

    let happy_path = async {
        let herc20_deployed = herc20::watch_for_deployed(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
        )
        .await
        .map_err(|_| AliceDeploy)?;

        let _herc20_funded = herc20::watch_for_funded(
            ethereum_connector,
            herc20_params.clone(),
            utc_start_of_swap,
            herc20_deployed.clone(),
        )
        .await
        .map_err(|_| AliceFund)?;

        let hbit_funded = bob.execute_fund(&hbit_params).await.map_err(|_| BobFund)?;

        let hbit_redeemed = hbit::watch_for_redeemed(
            bitcoin_connector,
            &hbit_params.shared,
            hbit_funded.location,
            utc_start_of_swap,
        )
        .await
        .map_err(|_| AliceRedeem(hbit_funded))?;

        let _herc20_redeem = bob
            .execute_redeem(
                herc20_params,
                hbit_redeemed.secret,
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .map_err(|_| BobRedeem)?;

        Ok(())
    };

    if let Err(AliceRedeem(hbit_funded)) = happy_path.await {
        bob.execute_refund(hbit_params, hbit_funded).await?;
    };

    Ok(())
}

#[derive(Debug, Clone, thiserror::Error)]
enum Herc20HbitAliceError {
    #[error("Alice failed to deploy.")]
    AliceDeploy,
    #[error("Alice failed to fund.")]
    AliceFund,
    #[error("Bob failed to fund.")]
    BobFund(herc20::Deployed),
    #[error("Alice failed to redeem.")]
    AliceRedeem(herc20::Deployed),
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
enum Herc20HbitBobError {
    #[error("Alice failed to deploy.")]
    AliceDeploy,
    #[error("Alice failed to fund.")]
    AliceFund,
    #[error("Bob failed to fund.")]
    BobFund,
    #[error("Alice failed to redeem.")]
    AliceRedeem(hbit::Funded),
    #[error("Bob failed to redeem.")]
    BobRedeem,
}
