use crate::swap::{
    comit::{Action, Error},
    hbit, herc20,
};
use comit::Secret;
use futures::Stream;
use genawaiter::sync::Gen;
use time::OffsetDateTime;

/// Execute a Hbit<->Herc20 swap for Alice.
#[allow(dead_code)] // This is library code
pub fn hbit_herc20_alice<A, B>(
    hbit: A,
    herc20: B,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    secret: Secret,
    utc_start_of_swap: OffsetDateTime,
) -> impl Stream<Item = Result<Action, Error<hbit::IncorrectlyFunded, herc20::IncorrectlyFunded>>>
where
    A: hbit::WatchForFunded + hbit::WatchForRedeemed,
    B: herc20::WatchForDeployed + herc20::WatchForFunded + herc20::WatchForRedeemed,
{
    Gen::new(|co| async move {
        tracing::info!("starting swap");

        co.yield_(Ok(Action::HbitFund(hbit_params.clone()))).await;
        let hbit_funded = match hbit.watch_for_funded(&hbit_params, utc_start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(e) => {
                co.yield_(Err(Error::AlphaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("we funded the hbit htlc");

        let herc20_deployed = herc20
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
            .await;

        tracing::info!("bob deployed the herc20 htlc");

        match herc20
            .watch_for_funded(herc20_params.clone(), herc20_deployed, utc_start_of_swap)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                co.yield_(Err(Error::BetaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("bob funded the herc20 htlc");

        co.yield_(Ok(Action::Herc20Redeem(
            herc20_params.clone(),
            herc20_deployed,
            secret,
        )))
        .await;
        let _ = herc20
            .watch_for_redeemed(herc20_params, herc20_deployed, utc_start_of_swap)
            .await;

        tracing::info!("we redeemed the herc20 htlc");

        let _ = hbit
            .watch_for_redeemed(&hbit_params, hbit_funded, utc_start_of_swap)
            .await;

        tracing::info!("bob redeemed the hbit htlc");
    })
}

/// Execute a Hbit<->Herc20 swap for Bob.
pub fn hbit_herc20_bob<A, B>(
    hbit: A,
    herc20: B,
    hbit_params: hbit::Params,
    herc20_params: herc20::Params,
    utc_start_of_swap: OffsetDateTime,
) -> impl Stream<Item = Result<Action, Error<hbit::IncorrectlyFunded, herc20::IncorrectlyFunded>>>
where
    A: hbit::WatchForFunded + hbit::WatchForRedeemed,
    B: herc20::WatchForDeployed + herc20::WatchForFunded + herc20::WatchForRedeemed,
{
    Gen::new(|co| async move {
        tracing::info!("starting swap");

        let hbit_funded = match hbit.watch_for_funded(&hbit_params, utc_start_of_swap).await {
            Ok(hbit_funded) => hbit_funded,
            Err(e) => {
                co.yield_(Err(Error::AlphaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("alice funded the hbit htlc");

        co.yield_(Ok(Action::Herc20Deploy(herc20_params.clone())))
            .await;
        let herc20_deployed = herc20
            .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
            .await;

        tracing::info!("we deployed the herc20 htlc");

        co.yield_(Ok(Action::Herc20Fund(
            herc20_params.clone(),
            herc20_deployed,
        )))
        .await;
        match herc20
            .watch_for_funded(herc20_params.clone(), herc20_deployed, utc_start_of_swap)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                co.yield_(Err(Error::BetaIncorrectlyFunded(e))).await;
                return;
            }
        };

        tracing::info!("we funded the herc20 htlc");

        let herc20_redeemed = herc20
            .watch_for_redeemed(herc20_params.clone(), herc20_deployed, utc_start_of_swap)
            .await;

        tracing::info!("alice redeemed the herc20 htlc");

        co.yield_(Ok(Action::HbitRedeem(
            hbit_params.clone(),
            hbit_funded,
            herc20_redeemed.secret,
        )))
        .await;
        let _ = hbit
            .watch_for_redeemed(&hbit_params, hbit_funded, utc_start_of_swap)
            .await;

        tracing::info!("we redeemed the hbit htlc");
    })
}
