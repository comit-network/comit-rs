use crate::swap::{hbit, herc20};
use chrono::NaiveDateTime;
use comit::{
    btsieve,
    btsieve::{BlockByHash, LatestBlock},
    ethereum, Secret,
};

/// Execute a Hbit<->Herc20 swap for Alice.
///
/// Delegates to `hbit_herc20_happy_alice` and handles errors by
/// executing refund for Alice when necessary.
#[allow(dead_code)] // This is library code
pub async fn hbit_herc20_alice<A, EC>(
    alice: A,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secret: Secret,
    utc_start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    A: hbit::ExecuteFund + herc20::ExecuteRedeem + hbit::ExecuteRefund,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    let res = hbit_herc20_happy_alice(
        &alice,
        ethereum_connector,
        hbit_params,
        herc20_params,
        secret,
        utc_start_of_swap,
    )
    .await;

    use HbitHerc20AliceError::*;
    if let Err(BobDeploy(hbit_funded)) | Err(BobFund(hbit_funded)) | Err(AliceRedeem(hbit_funded)) =
        res
    {
        alice.execute_refund(hbit_params, hbit_funded).await?;
    };

    Ok(())
}

/// Execute the happy path of a Hbit<->Herc20 swap for Alice.
async fn hbit_herc20_happy_alice<A, EC>(
    alice: &A,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secret: Secret,
    utc_start_of_swap: NaiveDateTime,
) -> Result<(), HbitHerc20AliceError>
where
    A: hbit::ExecuteFund + herc20::ExecuteRedeem,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    use HbitHerc20AliceError::*;

    let hbit_funded = alice
        .execute_fund(&hbit_params)
        .await
        .map_err(|_| AliceFund)?;

    let herc20_deployed =
        herc20::watch_for_deployed(ethereum_connector, herc20_params.clone(), utc_start_of_swap)
            .await
            .map_err(|_| BobDeploy(hbit_funded))?;

    let _herc20_funded = herc20::watch_for_funded(
        ethereum_connector,
        herc20_params.clone(),
        utc_start_of_swap,
        herc20_deployed.clone(),
    )
    .await
    .map_err(|_| BobFund(hbit_funded))?;

    let _herc20_redeemed = alice
        .execute_redeem(herc20_params, secret, herc20_deployed, utc_start_of_swap)
        .await
        .map_err(|_| AliceRedeem(hbit_funded))?;

    Ok(())
}

/// Execute a Hbit<->Herc20 swap for Bob.
///
/// Delegates to `hbit_herc20_happy_bob` and handles errors by
/// executing refund for Bob when necessary.
pub async fn hbit_herc20_bob<B, BC, EC>(
    bob: B,
    bitcoin_connector: &BC,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    utc_start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    B: herc20::ExecuteDeploy + herc20::ExecuteFund + hbit::ExecuteRedeem + herc20::ExecuteRefund,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    let res = hbit_herc20_happy_bob(
        &bob,
        bitcoin_connector,
        ethereum_connector,
        hbit_params,
        herc20_params.clone(),
        utc_start_of_swap,
    )
    .await;

    use HbitHerc20BobError::*;
    if let Err(AliceRedeem(herc20_deployed)) = res {
        bob.execute_refund(herc20_params, herc20_deployed, utc_start_of_swap)
            .await?;
    }

    Ok(())
}

/// Execute the happy path of a Hbit<->Herc20 swap for Bob.
async fn hbit_herc20_happy_bob<B, BC, EC>(
    bob: &B,
    bitcoin_connector: &BC,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    utc_start_of_swap: NaiveDateTime,
) -> Result<(), HbitHerc20BobError>
where
    B: herc20::ExecuteDeploy + herc20::ExecuteFund + hbit::ExecuteRedeem,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    use HbitHerc20BobError::*;

    let hbit_funded =
        hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, utc_start_of_swap)
            .await
            .map_err(|_| AliceFund)?;

    let herc20_deployed = bob
        .execute_deploy(herc20_params.clone())
        .await
        .map_err(|_| BobDeploy)?;

    let _herc20_funded = bob
        .execute_fund(
            herc20_params.clone(),
            herc20_deployed.clone(),
            utc_start_of_swap,
        )
        .await
        .map_err(|_| BobFund)?;

    let herc20_redeemed = herc20::watch_for_redeemed(
        ethereum_connector,
        utc_start_of_swap,
        herc20_deployed.clone(),
    )
    .await
    .map_err(|_| AliceRedeem(herc20_deployed))?;

    let _hbit_redeem = bob
        .execute_redeem(hbit_params, hbit_funded, herc20_redeemed.secret)
        .await
        .map_err(|_| BobRedeem)?;

    dbg!(_hbit_redeem);

    Ok(())
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
enum HbitHerc20AliceError {
    #[error("Alice failed to fund.")]
    AliceFund,
    #[error("Bob failed to deploy.")]
    BobDeploy(hbit::Funded),
    #[error("Bob failed to fund.")]
    BobFund(hbit::Funded),
    #[error("Alice failed to redeem.")]
    AliceRedeem(hbit::Funded),
}

#[derive(Debug, Clone, thiserror::Error)]
enum HbitHerc20BobError {
    #[error("Alice failed to fund.")]
    AliceFund,
    #[error("Bob failed to deploy.")]
    BobDeploy,
    #[error("Bob failed to fund.")]
    BobFund,
    #[error("Alice failed to redeem.")]
    AliceRedeem(herc20::Deployed),
    #[error("Bob failed to redeem.")]
    BobRedeem,
}
