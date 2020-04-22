use crate::swap_protocols::{
    halight::{
        self, Params, States, WaitForAccepted, WaitForCancelled, WaitForOpened, WaitForSettled,
    },
    rfc003::SecretHash,
    state::Update,
    LocalSwapId,
};
use futures::TryStreamExt;
use std::sync::Arc;

/// Creates a new instance of the halight protocol.
///
/// This function delegates to the `halight` module for the actual protocol
/// implementation. Its main purpose is to annotate the protocol instance with
/// logging information and store the events yielded by the protocol in
/// `halight::States`.
pub async fn new_halight_swap<C>(
    id: LocalSwapId,
    secret_hash: SecretHash,
    state_store: Arc<States>,
    connector: C,
) where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    let mut events = halight::new(&connector, Params { secret_hash })
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        state_store.update(&id, event).await;
    }

    tracing::info!("swap finished");
}
