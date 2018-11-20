use comit_client::{self, SwapReject, SwapResponseError};
use event_store::EventStore;
use futures::{
    stream::Stream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    Future,
};
use key_store::KeyStore;
use rand::thread_rng;
use std::{marker::PhantomData, net::SocketAddr, sync::Arc};
use swap_protocols::{
    asset::Asset,
    metadata_store::MetadataStore,
    rfc003::{
        self,
        alice::SwapRequestKind,
        roles::Alice,
        state_machine::{Start, SwapStates},
        state_store::StateStore,
        Ledger, Secret,
    },
};
use swaps::{alice_events, common::SwapId};

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
    pub alice_actor_sender: UnboundedSender<SwapId>,
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

        let event_store = Arc::clone(&self.event_store);
        let alice_actor_sender = self.alice_actor_sender.clone();
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

                        // This is legacy code
                        send_swap_request(
                            id,
                            comit_client::rfc003::Request {
                                alpha_asset: start_state.alpha_asset,
                                beta_asset: start_state.beta_asset,
                                alpha_ledger: start_state.alpha_ledger,
                                beta_ledger: start_state.beta_ledger,
                                alpha_ledger_refund_identity: start_state
                                    .alpha_ledger_refund_identity
                                    .into(),
                                beta_ledger_success_identity: start_state
                                    .beta_ledger_success_identity
                                    .into(),
                                alpha_ledger_lock_duration: start_state.alpha_ledger_lock_duration,
                                secret_hash: start_state.secret.hash(),
                            },
                            Arc::clone(&event_store),
                            alice_actor_sender.clone(),
                            Arc::clone(&client_factory),
                            comit_node_addr.clone(),
                            secret,
                        );

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

fn send_swap_request<
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
    C: comit_client::Client,
    F: comit_client::ClientFactory<C> + 'static,
    E: EventStore<SwapId>,
>(
    id: SwapId,
    swap_request: comit_client::rfc003::Request<AL, BL, AA, BA>,
    event_store: Arc<E>,
    alice_actor_sender: UnboundedSender<SwapId>,
    client_factory: Arc<F>,
    comit_node_addr: SocketAddr,
    secret: Secret,
) {
    let sent_event = alice_events::SentSwapRequest {
        alpha_ledger: swap_request.alpha_ledger.clone(),
        beta_ledger: swap_request.beta_ledger.clone(),
        alpha_asset: swap_request.alpha_asset.clone(),
        beta_asset: swap_request.beta_asset.clone(),
        secret,
        beta_ledger_success_identity: swap_request.beta_ledger_success_identity.clone(),
        alpha_ledger_refund_identity: swap_request.alpha_ledger_refund_identity.clone(),
        alpha_ledger_lock_duration: swap_request.alpha_ledger_lock_duration.clone(),
    };

    // This is legacy code, unwraps are fine

    event_store.add_event(id, sent_event).unwrap();

    let client = client_factory.client_for(comit_node_addr).unwrap();

    let future = client
        .send_swap_request(swap_request)
        .then(move |response| {
            on_swap_response::<AL, BL, AA, BA, E>(id, &event_store, alice_actor_sender, response);
            Ok(())
        });

    tokio::spawn(future);
}

fn on_swap_response<
    AL: rfc003::Ledger,
    BL: rfc003::Ledger,
    AA: Clone + Send + Sync + 'static,
    BA: Clone + Send + Sync + 'static,
    E: EventStore<SwapId>,
>(
    id: SwapId,
    event_store: &Arc<E>,
    alice_actor_sender: UnboundedSender<SwapId>,
    result: Result<
        Result<comit_client::rfc003::AcceptResponseBody<AL, BL>, SwapReject>,
        SwapResponseError,
    >,
) {
    match result {
        Ok(Ok(accepted)) => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestAccepted::<AL, BL, AA, BA>::new(
                        accepted.beta_ledger_refund_identity,
                        accepted.alpha_ledger_success_identity,
                        accepted.beta_ledger_lock_duration,
                    ),
                )
                .expect("It should not be possible to be in the wrong state");

            alice_actor_sender
                .unbounded_send(id)
                .expect("Receiver should always be in scope");
        }
        _ => {
            event_store
                .add_event(
                    id,
                    alice_events::SwapRequestRejected::<AL, BL, AA, BA>::new(),
                )
                .expect("It should not be possible to be in the wrong state");
        }
    }
}
