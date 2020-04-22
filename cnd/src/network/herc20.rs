use crate::{
    asset,
    btsieve::{
        bitcoin::{self, BitcoindConnector},
        ethereum::{self, Cache, Web3Connector},
    },
    herc20::{
        self, LndConnectorAsReceiver, LndConnectorAsSender, LndConnectorParams, Params, States,
        WaitForAccepted, WaitForCancelled, WaitForOpened, WaitForSettled,
    },
    htlc_location, identity,
    swap_protocols::{ledger, rfc003::create_swap::HtlcParams, LedgerStates, LocalSwapId, Role},
    transaction,
};
use chrono::Utc;
use std::sync::Arc;

/// Creates a new instance of the herc20 protocol.
///
/// This function delegates to the `herc20` module for the actual protocol
/// implementation. Its main purpose is to annotate the protocol instance with
/// logging information and store the events yielded by the protocol in
/// `herc20::States`.
#[allow(dead_code)]
async fn new_herc20_swap<C>(
    local_swap_id: LocalSwapId,
    params: Params,
    state_store: Arc<States>,
    connector: C,
) where
    C: WaitForDeployed + WaitForFunded + WaitForRedeemed + WaitForRefunded,
{
    let mut events = herc20::new(&connector, params)
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        state_store.update(&SwapId(local_swap_id.0), event).await;
    }

    tracing::info!("swap finished");
}
