use comit_client;
use event_store::EventStore;
use futures::{stream::Stream, sync::mpsc::UnboundedReceiver, Future};
use key_store::KeyStore;
use rand::thread_rng;
use std::{marker::PhantomData, net::SocketAddr, sync::Arc};
use swap_protocols::{
    asset::Asset,
    metadata_store::MetadataStore,
    rfc003::{
        alice::SwapRequestKind,
        roles::Alice,
        state_machine::{Start, SwapStates},
        state_store::StateStore,
        Ledger, Secret,
    },
};
use swaps::common::SwapId;

#[derive(Debug)]
pub struct SwapRequestHandler<
    C: comit_client::Client,
    F: comit_client::ClientFactory<C> + 'static,
    EventStore,
    MetadataStore,
    StateStore,
> {
    // new dependencies
    pub receiver: UnboundedReceiver<(SwapId, SwapRequestKind)>,
    pub metadata_store: Arc<MetadataStore>,
    pub key_store: Arc<KeyStore>,
    pub state_store: Arc<StateStore>,

    // legacy code dependencies
    pub client_factory: Arc<F>,
    pub event_store: Arc<EventStore>,
    pub comit_node_addr: SocketAddr,
    pub phantom_data: PhantomData<C>,
}

impl<
        C: comit_client::Client,
        F: comit_client::ClientFactory<C> + 'static,
        E: EventStore<SwapId>,
        M: MetadataStore<SwapId>,
        S: StateStore<SwapId>,
    > SwapRequestHandler<C, F, E, M, S>
{
    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let receiver = self.receiver;
        let key_store = Arc::clone(&self.key_store);
        let metadata_store = Arc::clone(&self.metadata_store);
        let state_store = Arc::clone(&self.state_store);

        let client_factory = Arc::clone(&self.client_factory);
        let comit_node_addr = self.comit_node_addr.clone();

        receiver
            .for_each(move |(id, requests)| {
                match requests {
                    SwapRequestKind::BitcoinEthereumBitcoinQuantityEthereumQuantity(request) => {
                        // TODO: Store this somewhere
                        let _alpha_ledger_refund_identity = request.alpha_ledger_refund_identity;

                        let alpha_ledger_refund_identity =
                            key_store.get_transient_keypair(&id.into(), b"REFUND");

                        if let Err(e) = metadata_store.insert(id, request.clone()) {
                            error!("Failed to store metadata for swap {} because {:?}", id, e);

                            // Return Ok to keep the loop running
                            return Ok(());
                        }

                        let secret = Secret::generate(&mut thread_rng());

                        let start_state = Start {
                            alpha_ledger_refund_identity,
                            beta_ledger_success_identity: request.beta_ledger_success_identity,
                            alpha_ledger: request.alpha_ledger,
                            beta_ledger: request.beta_ledger,
                            alpha_asset: request.alpha_asset,
                            beta_asset: request.beta_asset,
                            alpha_ledger_lock_duration: request.alpha_ledger_lock_duration,
                            secret,
                        };

                        spawn_state_machine(id, start_state.clone(), state_store.as_ref());
                        Ok(())
                    }
                }
            })
            .map_err(|_| ())
    }
}

fn spawn_state_machine<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, S: StateStore<SwapId>>(
    id: SwapId,
    start_state: Start<Alice<AL, BL, AA, BA>>,
    state_store: &S,
) {
    let state = SwapStates::Start(start_state);

    // TODO: spawn state machine from state here

    state_store.insert(id, state).expect("handle errors :)");
}
