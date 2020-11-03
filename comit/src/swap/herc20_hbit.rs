use crate::{
    swap::{hbit, herc20, Action, Error},
    Secret,
};
use bitcoin::secp256k1::{Secp256k1, Signing};
use futures::Stream;
use genawaiter::sync::Gen;
use time::OffsetDateTime;

/// Execute a Herc20<->Hbit swap for Alice.
pub fn herc20_hbit_alice<A, B, C>(
    herc20: A,
    hbit: B,
    secp: Secp256k1<C>,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    secret: Secret,
    utc_start_of_swap: OffsetDateTime,
) -> impl Stream<Item = Result<Action, Error<herc20::IncorrectlyFunded, hbit::IncorrectlyFunded>>>
where
    A: herc20::WatchForDeployed + herc20::WatchForFunded + herc20::WatchForRedeemed,
    B: hbit::WatchForRedeemed + hbit::WatchForFunded,
    C: Signing,
{
    Gen::new(|co| async move {
        tracing::info!("starting swap");

        co.yield_(Ok(Action::Herc20Deploy(
            herc20_params.build_deploy_action(),
        )))
        .await;
        let herc20_deployed = herc20
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
            .await;

        tracing::info!("we deployed the herc20 htlc");

        co.yield_(Ok(Action::Herc20Fund(
            herc20_params.build_fund_action(herc20_deployed.location),
        )))
        .await;
        match herc20
            .watch_for_funded(herc20_params.clone(), herc20_deployed, utc_start_of_swap)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                co.yield_(Err(Error::AlphaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("we funded the herc20 htlc");

        let hbit_funded = match hbit.watch_for_funded(&hbit_params, utc_start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(e) => {
                co.yield_(Err(Error::BetaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("bob funded the hbit htlc");

        co.yield_(Ok(Action::HbitRedeem(
            hbit_params.build_redeem_action(&secp, hbit_funded.location, secret),
            secret,
        )))
        .await;
        let _ = hbit
            .watch_for_redeemed(&hbit_params, hbit_funded, utc_start_of_swap)
            .await;

        tracing::info!("we redeemed the hbit htlc");

        let _ = herc20
            .watch_for_redeemed(herc20_params, herc20_deployed, utc_start_of_swap)
            .await;

        tracing::info!("bob redeemed the herc20 htlc");
    })
}

/// Execute a Herc20<->Hbit swap for Bob.
pub fn herc20_hbit_bob<A, B>(
    herc20: A,
    hbit: B,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: OffsetDateTime,
) -> impl Stream<Item = Result<Action, Error<herc20::IncorrectlyFunded, hbit::IncorrectlyFunded>>>
where
    A: herc20::WatchForDeployed + herc20::WatchForFunded + herc20::WatchForRedeemed,
    B: hbit::WatchForRedeemed + hbit::WatchForFunded,
{
    Gen::new(|co| async move {
        tracing::info!("starting swap");

        let herc20_deployed = herc20
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
            .await;

        tracing::info!("alice deployed the herc20 htlc");

        match herc20
            .watch_for_funded(herc20_params.clone(), herc20_deployed, utc_start_of_swap)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                co.yield_(Err(Error::AlphaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("alice funded the herc20 htlc");

        co.yield_(Ok(Action::HbitFund(hbit_params.build_fund_action())))
            .await;
        let hbit_funded = match hbit.watch_for_funded(&hbit_params, utc_start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(e) => {
                co.yield_(Err(Error::BetaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("we funded the hbit htlc");

        let hbit_redeemed = hbit
            .watch_for_redeemed(&hbit_params, hbit_funded, utc_start_of_swap)
            .await;

        tracing::info!("alice redeemed the hbit htlc");

        co.yield_(Ok(Action::Herc20Redeem(
            herc20_params.build_redeem_action(herc20_deployed.location, hbit_redeemed.secret),
            hbit_redeemed.secret,
        )))
        .await;
        let _ = herc20
            .watch_for_redeemed(herc20_params, herc20_deployed, utc_start_of_swap)
            .await;

        tracing::info!("we redeemed the herc20 htlc");
    })
}
