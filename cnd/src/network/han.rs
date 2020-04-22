use crate::{
    asset,
    btsieve::ethereum::{Cache, Web3Connector},
    htlc_location, identity,
    swap_protocols::{
        han, ledger, rfc003::create_swap::HtlcParams, LedgerStates, LocalSwapId, Role,
    },
    transaction,
};
use chrono::Utc;
use std::sync::Arc;
use tracing_futures::Instrument;

pub async fn new_han_ethereum_ether_swap(
    swap_id: LocalSwapId,
    connector: Arc<Cache<Web3Connector>>,
    ethereum_ledger_state: Arc<LedgerStates>,
    htlc_params: HtlcParams<ledger::Ethereum, asset::Ether, identity::Ethereum>,
    role: Role,
) {
    han::create_watcher::<_, _, _, _, htlc_location::Ethereum, _, transaction::Ethereum>(
        connector.as_ref(),
        ethereum_ledger_state,
        swap_id,
        htlc_params,
        Utc::now().naive_local(),
    )
    .instrument(tracing::error_span!("alpha_ledger", swap_id = %swap_id, role = %role))
    .await
}
