use crate::{
    connectors::Connectors, halbit, hbit, herc20, storage::Storage, LocalSwapId, Role, Side,
};
use anyhow::Result;
use time::OffsetDateTime;
use tokio::runtime::Handle;

/// ProtocolSpawner acts as a bundle for all dependencies needed to spawn
/// instances of a protocol.
#[derive(Debug, Clone)]
pub struct ProtocolSpawner {
    connectors: Connectors,
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
    ) -> Result<()>;
}

impl ProtocolSpawner {
    pub fn new(connectors: Connectors, runtime_handle: Handle, storage: Storage) -> Self {
        Self {
            connectors,
            runtime_handle,
            storage,
        }
    }

    pub fn supports_halbit(&self) -> anyhow::Result<()> {
        self.connectors.supports_halbit()
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
    ) -> Result<()> {
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

        Ok(())
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
    ) -> Result<()> {
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

        Ok(())
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
    ) -> Result<()> {
        match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => {
                let task = halbit::new(
                    id,
                    params,
                    role,
                    side,
                    self.storage.clone(),
                    self.connectors.lnd_as_sender()?,
                );

                self.runtime_handle.spawn(task);
            }
            (Role::Bob, Side::Alpha) | (Role::Alice, Side::Beta) => {
                let task = halbit::new(
                    id,
                    params,
                    role,
                    side,
                    self.storage.clone(),
                    self.connectors.lnd_as_receiver()?,
                );

                self.runtime_handle.spawn(task);
            }
        }

        Ok(())
    }
}
