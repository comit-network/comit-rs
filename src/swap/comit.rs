//! Code that could be upstreamed to COMIT lib.

pub mod hbit;
pub mod herc20;

use chrono::NaiveDateTime;
use comit::btsieve::{BlockByHash, LatestBlock};
use futures::{
    future::{self, Either},
    pin_mut,
};

pub use comit::{ethereum, *};

/// Execute Alice's side of a Hbit<->Herc20 swap.
#[cfg(test)]
pub async fn hbit_herc20_alice<A, BC, EC>(
    alice: A,
    bitcoin_connector: &BC,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secret: comit::Secret,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    A: hbit::ExecuteFund + hbit::ExecuteRefund + herc20::ExecuteRedeem,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    let hbit_funded = match alice.execute_fund(&hbit_params).await {
        Ok(hbit_funded) => hbit_funded,
        Err(_) => return Ok(()),
    };

    let herc20_deployed =
        match herc20::watch_for_deployed(ethereum_connector, herc20_params.clone(), start_of_swap)
            .await
        {
            Ok(herc20_deployed) => herc20_deployed,
            Err(_) => {
                alice.execute_refund(hbit_params, hbit_funded).await?;

                return Ok(());
            }
        };

    let _herc20_funded = match herc20::watch_for_funded(
        ethereum_connector,
        herc20_params.clone(),
        start_of_swap,
        herc20_deployed.clone(),
    )
    .await
    {
        Ok(herc20_funded) => herc20_funded,
        Err(_) => {
            alice.execute_refund(hbit_params, hbit_funded).await?;

            return Ok(());
        }
    };

    let _herc20_redeemed = match alice
        .execute_redeem(herc20_params, secret, herc20_deployed, start_of_swap)
        .await
    {
        Ok(herc20_redeemed) => herc20_redeemed,
        Err(_) => {
            alice.execute_refund(hbit_params, hbit_funded).await?;

            return Ok(());
        }
    };

    let hbit_redeem = hbit::watch_for_redeemed(
        bitcoin_connector,
        &hbit_params.shared,
        hbit_funded.location,
        start_of_swap,
    );
    let hbit_refund = alice.execute_refund(hbit_params, hbit_funded);

    pin_mut!(hbit_redeem);
    pin_mut!(hbit_refund);

    match future::select(hbit_redeem, hbit_refund).await {
        Either::Left((Ok(_hbit_redeemed), _)) => Ok(()),
        Either::Right((Ok(_hbit_refunded), _)) => Ok(()),
        Either::Left((Err(_), hbit_refund)) => {
            hbit_refund.await?;
            Ok(())
        }
        Either::Right((Err(_), _hbit_redeem)) => Ok(()),
    }
}

/// Execute Bob's side of a Hbit<->Herc20 swap.
pub async fn hbit_herc20_bob<B, BC, EC>(
    bob: B,
    bitcoin_connector: &BC,
    ethereum_connector: &EC,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    B: herc20::ExecuteDeploy + herc20::ExecuteFund + herc20::ExecuteRefund + hbit::ExecuteRedeem,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
{
    let hbit_funded =
        match hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(_) => return Ok(()),
        };

    let herc20_deployed = match bob.execute_deploy(herc20_params.clone()).await {
        Ok(herc20_deployed) => herc20_deployed,
        Err(_) => return Ok(()),
    };

    let _herc20_funded = match bob
        .execute_fund(
            herc20_params.clone(),
            herc20_deployed.clone(),
            start_of_swap,
        )
        .await
    {
        Ok(herc20_funded) => herc20_funded,
        Err(_) => return Ok(()),
    };

    let herc20_redeemed = match herc20::watch_for_redeemed(
        ethereum_connector,
        start_of_swap,
        herc20_deployed.clone(),
    )
    .await
    {
        Ok(herc20_redeemed) => herc20_redeemed,
        Err(_) => {
            bob.execute_refund(herc20_params, herc20_deployed, start_of_swap)
                .await?;

            return Ok(());
        }
    };

    let hbit_redeem = bob.execute_redeem(hbit_params, hbit_funded, herc20_redeemed.secret);
    let hbit_refund = hbit::watch_for_refunded(
        bitcoin_connector,
        &hbit_params.shared,
        hbit_funded.location,
        start_of_swap,
    );

    pin_mut!(hbit_redeem);
    pin_mut!(hbit_refund);

    match future::select(hbit_redeem, hbit_refund).await {
        Either::Left((Ok(_hbit_redeemed), _)) => Ok(()),
        Either::Right((Ok(_hbit_refunded), _)) => Ok(()),
        Either::Left((Err(_), _hbit_refund)) => Ok(()),
        Either::Right((Err(_), hbit_redeem)) => {
            hbit_redeem.await?;
            Ok(())
        }
    }
}

/// Execute Alice's side of a Herc20<->Hbit swap.
pub async fn _herc20_hbit_alice<A, EC, BC>(
    alice: A,
    ethereum_connector: &EC,
    bitcoin_connector: &BC,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    secret: Secret,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    A: herc20::ExecuteDeploy + herc20::ExecuteFund + herc20::ExecuteRefund + hbit::ExecuteRedeem,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
{
    let herc20_deployed = match alice.execute_deploy(herc20_params.clone()).await {
        Ok(herc20_deployed) => herc20_deployed,
        Err(_) => return Ok(()),
    };

    let _herc20_funded = match alice
        .execute_fund(
            herc20_params.clone(),
            herc20_deployed.clone(),
            start_of_swap,
        )
        .await
    {
        Ok(herc20_funded) => herc20_funded,
        Err(_) => return Ok(()),
    };

    let hbit_funded =
        match hbit::watch_for_funded(bitcoin_connector, &hbit_params.shared, start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(_) => {
                alice
                    .execute_refund(
                        herc20_params.clone(),
                        herc20_deployed.clone(),
                        start_of_swap,
                    )
                    .await?;

                return Ok(());
            }
        };

    let _hbit_redeemed = match alice.execute_redeem(hbit_params, hbit_funded, secret).await {
        Ok(hbit_redeemed) => hbit_redeemed,
        Err(_) => {
            alice
                .execute_refund(
                    herc20_params.clone(),
                    herc20_deployed.clone(),
                    start_of_swap,
                )
                .await?;

            return Ok(());
        }
    };

    let herc20_redeem =
        herc20::watch_for_redeemed(ethereum_connector, start_of_swap, herc20_deployed.clone());
    let herc20_refund = alice.execute_refund(
        herc20_params.clone(),
        herc20_deployed.clone(),
        start_of_swap,
    );

    pin_mut!(herc20_redeem);
    pin_mut!(herc20_refund);

    match future::select(herc20_redeem, herc20_refund).await {
        Either::Left((Ok(_herc20_redeemed), _)) => Ok(()),
        Either::Right((Ok(_herc20_refunded), _)) => Ok(()),
        Either::Left((Err(_), herc20_refund)) => {
            herc20_refund.await?;
            Ok(())
        }
        Either::Right((Err(_), _herc20_redeem)) => Ok(()),
    }
}

/// Execute Bob's side of a Herc20<->Hbit swap.
pub async fn herc20_hbit_bob<B, EC, BC>(
    bob: B,
    ethereum_connector: &EC,
    bitcoin_connector: &BC,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    B: hbit::ExecuteFund + hbit::ExecuteRefund + herc20::ExecuteRedeem,
    EC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + btsieve::ethereum::ReceiptByHash,
    BC: LatestBlock<Block = ::bitcoin::Block>
        + BlockByHash<Block = ::bitcoin::Block, BlockHash = ::bitcoin::BlockHash>,
{
    let herc20_deployed =
        match herc20::watch_for_deployed(ethereum_connector, herc20_params.clone(), start_of_swap)
            .await
        {
            Ok(herc20_deployed) => herc20_deployed,
            Err(_) => return Ok(()),
        };

    let _herc20_funded = match herc20::watch_for_funded(
        ethereum_connector,
        herc20_params.clone(),
        start_of_swap,
        herc20_deployed.clone(),
    )
    .await
    {
        Ok(herc20_funded) => herc20_funded,
        Err(_) => return Ok(()),
    };

    let hbit_funded = match bob.execute_fund(&hbit_params).await {
        Ok(hbit_funded) => hbit_funded,
        Err(_) => return Ok(()),
    };

    let hbit_redeemed = match hbit::watch_for_redeemed(
        bitcoin_connector,
        &hbit_params.shared,
        hbit_funded.location,
        start_of_swap,
    )
    .await
    {
        Ok(hbit_redeemed) => hbit_redeemed,
        Err(_) => {
            bob.execute_refund(hbit_params, hbit_funded).await?;

            return Ok(());
        }
    };

    let herc20_redeem = bob.execute_redeem(
        herc20_params,
        hbit_redeemed.secret,
        herc20_deployed.clone(),
        start_of_swap,
    );
    let herc20_refund =
        herc20::watch_for_refunded(ethereum_connector, start_of_swap, herc20_deployed);

    pin_mut!(herc20_redeem);
    pin_mut!(herc20_refund);

    match future::select(herc20_redeem, herc20_refund).await {
        Either::Left((Ok(_herc20_redeemed), _)) => Ok(()),
        Either::Right((Ok(_herc20_refunded), _)) => Ok(()),
        Either::Left((Err(_), _herc20_refund)) => Ok(()),
        Either::Right((Err(_), herc20_redeem)) => {
            herc20_redeem.await?;
            Ok(())
        }
    }
}
