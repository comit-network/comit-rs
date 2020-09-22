use crate::{
    bitcoin,
    command::{into_history_trade, FinishedSwap},
    config::Settings,
    ethereum,
    history::History,
    swap::{Database, SwapExecutor},
};
use comit::btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use futures::{future::TryFutureExt, StreamExt};
use std::sync::Arc;

pub async fn resume_only(
    settings: Settings,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
) -> anyhow::Result<()> {
    #[cfg(not(test))]
    let db = Database::new(&settings.data.dir.join("database"))?;
    #[cfg(test)]
    let db = Database::new_test()?;
    let db = Arc::new(db);
    let mut history = History::new(settings.data.dir.join("history.csv").as_path())?;

    let (executor, mut finished_swap_receiver) = SwapExecutor::new(
        db.clone(),
        Arc::new(bitcoin_wallet),
        Arc::new(ethereum_wallet),
        Arc::new(BitcoindConnector::new(settings.bitcoin.bitcoind.node_url)?),
        Arc::new(Web3Connector::new(settings.ethereum.node_url)),
    );

    for swap in db.all_swaps()? {
        executor.execute(swap);
    }

    while let Some(finished_swap) = finished_swap_receiver.next().await {
        handle_finished_swap(finished_swap, db.as_ref(), &mut history)
    }

    Ok(())
}

fn handle_finished_swap(finished_swap: FinishedSwap, db: &Database, history: &mut History) {
    let trade = into_history_trade(
        finished_swap.peer.peer_id(),
        finished_swap.swap.clone(),
        #[cfg(not(test))]
        finished_swap.final_timestamp,
    );

    let _ = history.write(trade).map_err(|error| {
        tracing::error!(
            "Unable to register history entry: {:#}; {:?}",
            error,
            finished_swap
        )
    });

    let swap_id = finished_swap.swap.swap_id();

    let _ = db
        .remove_active_peer(&finished_swap.peer)
        .map_err(|error| tracing::error!("Unable to remove from active peers: {:#}", error));

    let _ = db
        .remove_swap(&swap_id)
        .map_err(|error| tracing::error!("Unable to delete swap from db: {:#}", error));
}
