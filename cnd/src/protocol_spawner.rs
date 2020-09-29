use crate::{
    connectors::Connectors, halbit, hbit, herc20, http_api::LedgerNotConfigured, storage::Storage,
    LocalSwapId, Role, Side,
};
use comit::lnd::{LndConnectorAsReceiver, LndConnectorAsSender, LndConnectorParams};
use time::OffsetDateTime;
use tokio::runtime::Handle;

/// ProtocolSpawner acts as a bundle for all dependencies needed to spawn
/// instances of a protocol.
#[derive(Debug, Clone)]
pub struct ProtocolSpawner {
    connectors: Connectors,
    lnd_connector_params: Option<LndConnectorParams>,
    runtime_handle: Handle,
    storage: Storage,
}

/// The `Spawn` trait abstracts over the functionality of spawning a particular
/// protocol given its params.
pub trait Spawn<P> {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: P,
        start_of_swap: OffsetDateTime,
        side: Side,
        role: Role,
    );
}

impl ProtocolSpawner {
    pub fn new(
        connectors: Connectors,
        lnd_connector_params: Option<LndConnectorParams>,
        runtime_handle: Handle,
        storage: Storage,
    ) -> Self {
        Self {
            connectors,
            lnd_connector_params,
            runtime_handle,
            storage,
        }
    }

    pub fn supports_halbit(&self) -> anyhow::Result<()> {
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
        start_of_swap: OffsetDateTime,
        side: Side,
        role: Role,
    ) {
        let task = herc20::new(
            id,
            params,
            start_of_swap,
            role,
            side,
            self.storage.clone(),
            self.connectors.ethereum(),
        );

        self.runtime_handle.spawn(task);
    }
}

impl Spawn<hbit::Params> for ProtocolSpawner {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: hbit::Params,
        start_of_swap: OffsetDateTime,
        side: Side,
        role: Role,
    ) {
        let task = hbit::new(
            id,
            params,
            start_of_swap,
            role,
            side,
            self.storage.clone(),
            self.connectors.bitcoin(),
        );

        self.runtime_handle.spawn(task);
    }
}

impl Spawn<halbit::Params> for ProtocolSpawner {
    fn spawn(
        &self,
        id: LocalSwapId,
        params: halbit::Params,
        _: OffsetDateTime,
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
                self.runtime_handle.spawn(halbit::new(
                    id,
                    params,
                    role,
                    side,
                    self.storage.clone(),
                    LndConnectorAsSender::from(lnd_connector_params.clone()),
                ));
            }
            (Role::Bob, Side::Alpha) | (Role::Alice, Side::Beta) => {
                self.runtime_handle.spawn(halbit::new(
                    id,
                    params,
                    role,
                    side,
                    self.storage.clone(),
                    LndConnectorAsReceiver::from(lnd_connector_params.clone()),
                ));
            }
        }
    }
}
