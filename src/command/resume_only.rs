use crate::{
    bitcoin,
    command::{into_history_trade, FinishedSwap},
    config::Settings,
    ethereum,
    history::History,
    swap::{Database, SwapKind},
};
use chrono::Utc;
use comit::btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use futures::future::{join_all, TryFutureExt};
use std::sync::{Arc, Mutex};

pub async fn resume_only(
    settings: Settings,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
) -> anyhow::Result<()> {
    let bitcoin_wallet = Arc::new(bitcoin_wallet);
    let ethereum_wallet = Arc::new(ethereum_wallet);

    #[cfg(not(test))]
    let db = Arc::new(Database::new(&settings.data.dir.join("database"))?);
    #[cfg(test)]
    let db = Arc::new(Database::new_test()?);

    let history = Arc::new(Mutex::new(History::new(
        settings.data.dir.join("history.csv").as_path(),
    )?));

    let bitcoin_connector = Arc::new(BitcoindConnector::new(settings.bitcoin.bitcoind.node_url)?);
    let ethereum_connector = Arc::new(Web3Connector::new(settings.ethereum.node_url));

    respawn_swaps(
        Arc::clone(&db),
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
        history,
    )
    .await?;

    Ok(())
}

async fn respawn_swaps(
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    history: Arc<Mutex<History>>,
) -> anyhow::Result<()> {
    let futures = db.all_swaps()?.into_iter().map(|swap| {
        execute_swap(
            Arc::clone(&db),
            Arc::clone(&bitcoin_wallet),
            Arc::clone(&ethereum_wallet),
            Arc::clone(&bitcoin_connector),
            Arc::clone(&ethereum_connector),
            swap,
        )
        .and_then(|finished_swap| async {
            handle_finished_swap(finished_swap, Arc::clone(&db), Arc::clone(&history));
            Ok(())
        })
    });

    join_all(futures).await;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn execute_swap(
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    swap: SwapKind,
) -> anyhow::Result<FinishedSwap> {
    swap.execute(
        Arc::clone(&db),
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
    )
    .await?;

    Ok(FinishedSwap::new(
        swap.clone(),
        swap.params().taker,
        Utc::now(),
    ))
}

fn handle_finished_swap(
    finished_swap: FinishedSwap,
    db: Arc<Database>,
    history: Arc<Mutex<History>>,
) {
    {
        let trade = into_history_trade(
            finished_swap.peer.peer_id(),
            finished_swap.swap.clone(),
            #[cfg(not(test))]
            finished_swap.final_timestamp,
        );

        let mut history = history
            .lock()
            .expect("No thread panicked while holding the lock");
        let _ = history.write(trade).map_err(|error| {
            tracing::error!(
                "Unable to register history entry: {}; {:?}",
                error,
                finished_swap
            )
        });
    }

    let swap_id = finished_swap.swap.swap_id();

    let _ = db
        .remove_active_peer(&finished_swap.peer)
        .map_err(|error| tracing::error!("Unable to remove from active peers: {}", error));

    let _ = db
        .remove_swap(&swap_id)
        .map_err(|error| tracing::error!("Unable to delete swap from db: {}", error));
}
