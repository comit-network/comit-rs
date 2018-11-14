use comit_client::{self, SwapReject, SwapResponseError};
use event_store::EventStore;
use futures::{
    stream::Stream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    Future,
};
use key_store::KeyStore;
use rand::thread_rng;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use swap_protocols::{
    asset::Asset,
    metadata_store::{Metadata, MetadataStore},
    rfc003::{
        self,
        alice_swap_request::AliceSwapRequests,
        messages::Request,
        state_machine::{Start, SwapStates},
        state_store::StateStore,
        Ledger, Secret,
    },
};
use swaps::{alice_events, common::SwapId};

#[derive(Debug)]
pub struct CreateSwap<
    C: comit_client::Client,
    F: comit_client::ClientFactory<C> + 'static,
    EventStore,
    MetadataStore,
    StateStore,
> {
    // new dependencies
    pub receiver: UnboundedReceiver<(SwapId, AliceSwapRequests)>,
    pub metadata_store: Arc<MetadataStore>,
    pub key_store: Arc<KeyStore>,
    pub state_store: Arc<StateStore>,

    // legacy code dependencies
    pub client_factory: Arc<F>,
    pub event_store: Arc<EventStore>,
    pub comit_node_addr: SocketAddr,
    pub alice_actor_sender: Arc<Mutex<UnboundedSender<SwapId>>>,
    pub phantom_data: PhantomData<C>,
}

impl<
        C: comit_client::Client,
        F: comit_client::ClientFactory<C> + 'static,
        E: EventStore<SwapId>,
        M: MetadataStore<SwapId>,
        S: StateStore<SwapId>,
    > CreateSwap<C, F, E, M, S>
{
    pub fn listen(self) -> impl Future<Item = (), Error = ()> {
        let receiver = self.receiver;
        let key_store = Arc::clone(&self.key_store);
        let metadata_store = Arc::clone(&self.metadata_store);
        let state_store = Arc::clone(&self.state_store);

        let event_store = Arc::clone(&self.event_store);
        let alice_actor_sender = Arc::clone(&self.alice_actor_sender);
        let client_factory = Arc::clone(&self.client_factory);
        let comit_node_addr = self.comit_node_addr.clone();

        receiver
            .for_each(move |(id, requests)| {
                match requests {
                    AliceSwapRequests::BitcoinEthereumBitcoinQuantityEthereumQuantity(request) => {
                        // TODO: Store this somewhere
                        let _source_ledger_refund_identity = request.source_ledger_refund_identity;

                        let source_ledger_refund_identity =
                            key_store.get_transient_keypair(&id.into(), b"REFUND");

                        store_metadata(id, request.clone(), metadata_store.as_ref());

                        let secret = Secret::generate(&mut thread_rng());

                        let start_state = Start {
                            source_ledger_refund_identity,
                            target_ledger_success_identity: request.target_ledger_success_identity,
                            source_ledger: request.source_ledger,
                            target_ledger: request.target_ledger,
                            source_asset: request.source_asset,
                            target_asset: request.target_asset,
                            source_ledger_lock_duration: request.source_ledger_lock_duration,
                            secret,
                        };

                        spawn_state_machine(id, start_state.clone(), state_store.as_ref());

                        // This is legacy code
                        send_swap_request(
                            id,
                            Request {
                                source_asset: start_state.source_asset,
                                target_asset: start_state.target_asset,
                                source_ledger: start_state.source_ledger,
                                target_ledger: start_state.target_ledger,
                                source_ledger_refund_identity: start_state
                                    .source_ledger_refund_identity
                                    .into(),
                                target_ledger_success_identity: start_state
                                    .target_ledger_success_identity
                                    .into(),
                                source_ledger_lock_duration: start_state
                                    .source_ledger_lock_duration,
                                secret_hash: start_state.secret.hash(),
                            },
                            Arc::clone(&event_store),
                            Arc::clone(&alice_actor_sender),
                            Arc::clone(&client_factory),
                            comit_node_addr.clone(),
                            secret,
                        )
                    }
                }
            })
            .map_err(|_| ())
    }
}

fn spawn_state_machine<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: StateStore<SwapId>>(
    id: SwapId,
    start_state: Start<SL, TL, SA, TA, Secret>,
    state_store: &S,
) {
    let state = SwapStates::Start(start_state);

    // TODO: spawn state machine from state here

    let _ = state_store.insert(id, state);
}

fn store_metadata<M: Into<Metadata>, MS: MetadataStore<SwapId>>(
    id: SwapId,
    metadata: M,
    metadata_store: &MS,
) {
    let _ = metadata_store.insert(id, metadata.into());
}

fn send_swap_request<
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    C: comit_client::Client,
    F: comit_client::ClientFactory<C> + 'static,
    E: EventStore<SwapId>,
>(
    id: SwapId,
    swap_request: Request<SL, TL, SA, TA>,
    event_store: Arc<E>,
    alice_actor_sender: Arc<Mutex<UnboundedSender<SwapId>>>,
    client_factory: Arc<F>,
    comit_node_addr: SocketAddr,
    secret: Secret,
) -> impl Future<Item = (), Error = ()> {
    let sent_event = alice_events::SentSwapRequest {
        source_ledger: swap_request.source_ledger.clone(),
        target_ledger: swap_request.target_ledger.clone(),
        source_asset: swap_request.source_asset.clone(),
        target_asset: swap_request.target_asset.clone(),
        secret,
        target_ledger_success_identity: swap_request.target_ledger_success_identity.clone(),
        source_ledger_refund_identity: swap_request.source_ledger_refund_identity.clone(),
        source_ledger_lock_duration: swap_request.source_ledger_lock_duration.clone(),
    };

    // This is legacy code, unwraps are fine

    event_store.add_event(id, sent_event).unwrap();

    let client = client_factory.client_for(comit_node_addr).unwrap();

    let response_future = client.send_swap_request(swap_request);

    response_future.then(move |response| {
        on_swap_response::<SL, TL, SA, TA, E>(id, &event_store, &alice_actor_sender, response);
        Ok(())
    })
}

fn on_swap_response<
    SL: rfc003::Ledger,
    TL: rfc003::Ledger,
    SA: Clone + Send + Sync + 'static,
    TA: Clone + Send + Sync + 'static,
    E: EventStore<SwapId>,
>(
    id: SwapId,
    event_store: &Arc<E>,
    alice_actor_sender: &Arc<Mutex<UnboundedSender<SwapId>>>,
    result: Result<Result<rfc003::AcceptResponseBody<SL, TL>, SwapReject>, SwapResponseError>,
) {
    use std::ops::DerefMut;

    match result {
        Ok(Ok(accepted)) => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestAccepted::<SL, TL, SA, TA>::new(
                        accepted.target_ledger_refund_identity,
                        accepted.source_ledger_success_identity,
                        accepted.target_ledger_lock_duration,
                    ),
                )
                .expect("It should not be possible to be in the wrong state");

            let mut alice_actor_sender = alice_actor_sender
                .lock()
                .expect("Issue with unlocking alice actor sender");
            let alice_actor_sender = alice_actor_sender.deref_mut();
            alice_actor_sender
                .unbounded_send(id)
                .expect("Receiver should always be in scope");
        }
        _ => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestRejected::<SL, TL, SA, TA>::new(),
                )
                .expect("It should not be possible to be in the wrong state");
        }
    }
}
