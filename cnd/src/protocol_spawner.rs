use crate::{
    btsieve, halight, hbit, herc20, http_api::LedgerNotConfigured, LocalSwapId, Role, Side,
};
use chrono::NaiveDateTime;
use comit::lnd::{LndConnectorAsReceiver, LndConnectorAsSender, LndConnectorParams};
use std::sync::Arc;
use tokio::runtime::Handle;

/// ProtocolSpawner acts as a bundle for all dependencies needed to spawn
/// instances of a protocol.
#[derive(Debug, Clone)]
pub struct ProtocolSpawner {
    ethereum_connector: Arc<btsieve::ethereum::Cache<btsieve::ethereum::Web3Connector>>,
    bitcoin_connector: Arc<btsieve::bitcoin::Cache<btsieve::bitcoin::BitcoindConnector>>,
    lnd_connector_params: Option<LndConnectorParams>,
    runtime_handle: Handle,

    herc20_states: Arc<herc20::States>,
    halight_states: Arc<halight::States>,
    hbit_states: Arc<hbit::States>,
}

/// The `Spawn` trait abstracts over the functionality of spawning a particular
/// protocol given its params.
pub trait Spawn<P> {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: P,
        start_of_swap: NaiveDateTime,
        side: Side,
        role: Role,
    );
}

impl ProtocolSpawner {
    pub fn new(
        ethereum_connector: Arc<btsieve::ethereum::Cache<btsieve::ethereum::Web3Connector>>,
        bitcoin_connector: Arc<btsieve::bitcoin::Cache<btsieve::bitcoin::BitcoindConnector>>,
        lnd_connector_params: Option<LndConnectorParams>,
        runtime_handle: Handle,
        herc20_states: Arc<herc20::States>,
        halight_states: Arc<halight::States>,
        hbit_states: Arc<hbit::States>,
    ) -> Self {
        Self {
            ethereum_connector,
            bitcoin_connector,
            lnd_connector_params,
            runtime_handle,
            herc20_states,
            halight_states,
            hbit_states,
        }
    }

    pub fn supports_halight(&self) -> anyhow::Result<()> {
        match self.lnd_connector_params {
            Some(_) => Ok(()),
            None => Err(anyhow::Error::from(LedgerNotConfigured {
                ledger: "lightning",
            })),
        }
    }
}

impl Spawn<herc20::Params> for ProtocolSpawner {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: herc20::Params,
        start_of_swap: NaiveDateTime,
        side: Side,
        role: Role,
    ) {
        let task = herc20::new(
            id,
            params,
            start_of_swap,
            role,
            side,
            self.herc20_states.clone(),
            self.ethereum_connector.clone(),
        );

        self.runtime_handle.spawn(task);
    }
}

impl Spawn<hbit::Params> for ProtocolSpawner {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: hbit::Params,
        start_of_swap: NaiveDateTime,
        side: Side,
        role: Role,
    ) {
        let task = hbit::new(
            id,
            params,
            start_of_swap,
            role,
            side,
            self.hbit_states.clone(),
            self.bitcoin_connector.clone(),
        );

        self.runtime_handle.spawn(task);
    }
}

impl Spawn<halight::Params> for ProtocolSpawner {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: halight::Params,
        _: NaiveDateTime,
        side: Side,
        role: Role,
    ) {
        let lnd_connector_params = match &self.lnd_connector_params {
            Some(params) => params,
            None => {
                tracing::warn!(
                    "failed to spawn swap {} because lnd connector params are not present",
                    id
                );
                return;
            }
        };

        match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => {
                self.runtime_handle.spawn(halight::new(
                    id,
                    params,
                    role,
                    side,
                    self.halight_states.clone(),
                    LndConnectorAsSender::from(lnd_connector_params.clone()),
                ));
            }
            (Role::Bob, Side::Alpha) | (Role::Alice, Side::Beta) => {
                self.runtime_handle.spawn(halight::new(
                    id,
                    params,
                    role,
                    side,
                    self.halight_states.clone(),
                    LndConnectorAsReceiver::from(lnd_connector_params.clone()),
                ));
            }
        }
    }
}
