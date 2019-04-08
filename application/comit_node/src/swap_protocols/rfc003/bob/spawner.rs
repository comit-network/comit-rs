use crate::swap_protocols::{
    asset::Asset,
    dependencies::{LedgerEventDependencies, ProtocolDependencies},
    metadata_store::{self, Metadata, MetadataStore, RoleKind},
    rfc003::{
        self, bob,
        create_ledger_events::CreateLedgerEvents,
        events::ResponseFuture,
        state_store::{self, StateStore},
        Ledger,
    },
    SwapId,
};
use futures::{sync::mpsc, Future, Stream};
use http_api_problem::HttpApiProblem;
use log::{error, info};
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Storage(state_store::Error),
    Metadata(metadata_store::Error),
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            Storage(e) => e.into(),
            Metadata(e) => e.into(),
        }
    }
}

pub trait BobSpawner: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<AL, BL>>, Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl<T: MetadataStore<SwapId>, S: StateStore> BobSpawner for ProtocolDependencies<T, S> {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<AL, BL>>, Error>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        let swap_seed = Arc::new(self.seed.swap_seed(id));
        let bob = bob::State::new(swap_request.clone(), swap_seed);

        let response_future = bob
            .response_future()
            .expect("This is always Some when Bob is created");

        self.metadata_store
            .insert(
                id,
                Metadata {
                    alpha_ledger: swap_request.alpha_ledger.into(),
                    beta_ledger: swap_request.beta_ledger.into(),
                    alpha_asset: swap_request.alpha_asset.into(),
                    beta_asset: swap_request.beta_asset.into(),
                    role: RoleKind::Bob,
                },
            )
            .map_err(Error::Metadata)?;

        let (sender, receiver) = mpsc::unbounded();

        let state_machine_future = bob.new_state_machine(
            self.ledger_events.create_ledger_events(),
            self.ledger_events.create_ledger_events(),
            Arc::new(sender),
        );

        let state_store = Arc::clone(&self.state_store);
        state_store.insert(id, bob);
        tokio::spawn(receiver.for_each(move |update| {
            state_store.update::<bob::State<AL, BL, AA, BA>>(id, update);
            Ok(())
        }));

        tokio::spawn(
            state_machine_future
                .map(move |outcome| {
                    info!("Swap {} finished with {:?}", id, outcome);
                })
                .map_err(move |e| {
                    error!("Swap {} failed with {:?}", id, e);
                }),
        );

        Ok(response_future)
    }
}
