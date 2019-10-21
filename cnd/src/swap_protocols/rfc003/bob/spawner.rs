use crate::swap_protocols::{
    asset::Asset,
    dependencies::{self, LedgerEventDependencies},
    rfc003::{
        self, bob, create_ledger_events::CreateLedgerEvents, events::ResponseFuture,
        state_store::StateStore, Ledger,
    },
    MetadataStore,
};
use futures::{sync::mpsc, Future, Stream};
use std::sync::Arc;

pub trait BobSpawner: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<AL, BL>>, ()>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl<T: MetadataStore, S: StateStore> BobSpawner for dependencies::bob::ProtocolDependencies<T, S> {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<AL, BL>>, ()>
    where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        let id = swap_request.id;
        let swap_seed = Arc::new(self.seed.swap_seed(id));
        let bob = bob::State::new(swap_request.clone(), swap_seed);

        let response_future = bob
            .response_future()
            .expect("This is always Some when Bob is created");

        let (sender, receiver) = mpsc::unbounded();

        let state_machine_future = bob.new_state_machine(
            self.ledger_events.create_ledger_events(),
            self.ledger_events.create_ledger_events(),
            Arc::new(sender),
        );

        let state_store = Arc::clone(&self.state_store);
        tokio::spawn(receiver.for_each(move |update| {
            state_store.update::<bob::State<AL, BL, AA, BA>>(&id, update);
            Ok(())
        }));

        tokio::spawn(
            state_machine_future
                .map(move |outcome| {
                    log::info!("Swap {} finished with {:?}", id, outcome);
                })
                .map_err(move |e| {
                    log::error!("Swap {} failed with {:?}", id, e);
                }),
        );

        Ok(response_future)
    }
}
