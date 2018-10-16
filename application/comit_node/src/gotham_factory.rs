use comit_client;
use comit_wallet::KeyStore;
use event_store;
use futures::sync::mpsc::UnboundedSender;
use gotham::{
    handler::HandlerFuture,
    middleware::Middleware,
    pipeline::{new_pipeline, single::single_pipeline},
    router::{builder::*, Router},
    state::{State, StateData},
};
use http_api;
use rand::OsRng;
use std::{
    net::SocketAddr,
    panic::RefUnwindSafe,
    sync::{Arc, Mutex},
};
use swaps::common::TradeId;

#[derive(Deserialize, StateData, StaticResponseExtender, Debug)]
pub struct SwapId {
    pub id: TradeId,
}

#[derive(Clone, NewMiddleware, Debug)]
struct SwapMiddleware<
    F: Clone + StateData + Sync + RefUnwindSafe,
    E: Clone + StateData + Sync + RefUnwindSafe,
> {
    pub swap_state: SwapState,
    pub event_store: E,
    pub client_factory: F,
}

#[derive(StateData, Clone, Debug)]
pub struct SwapState {
    pub rng: Arc<Mutex<OsRng>>,
    pub remote_comit_node_socket_addr: SocketAddr,
    pub key_store: Arc<KeyStore>,
    pub alice_actor_sender: Arc<Mutex<UnboundedSender<TradeId>>>,
}

#[derive(StateData, Debug)]
pub struct ClientFactory<C: 'static>(pub Arc<comit_client::ClientFactory<C>>);

impl<C> Clone for ClientFactory<C> {
    fn clone(&self) -> Self {
        ClientFactory(self.0.clone())
    }
}

#[derive(Debug)]
pub struct EventStore<E>(pub Arc<E>);

impl<E: event_store::EventStore<TradeId>> StateData for EventStore<E> {}
impl<E> Clone for EventStore<E> {
    fn clone(&self) -> Self {
        EventStore(self.0.clone())
    }
}

impl<F: StateData + Clone + Sync + RefUnwindSafe, E: StateData + Clone + Sync + RefUnwindSafe>
    Middleware for SwapMiddleware<F, E>
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.swap_state);
        state.put(self.client_factory);
        state.put(self.event_store);
        chain(state)
    }
}

pub fn create_gotham_router<
    C: comit_client::Client + 'static,
    F: comit_client::ClientFactory<C> + 'static,
    E: event_store::EventStore<TradeId> + RefUnwindSafe,
>(
    event_store: Arc<E>,
    client_factory: Arc<F>,
    remote_comit_node_socket_addr: SocketAddr,
    key_store: Arc<KeyStore>,
    alice_actor_sender: UnboundedSender<TradeId>,
) -> Router {
    let rng = Arc::new(Mutex::new(
        OsRng::new().expect("Failed to get randomness from OS"),
    ));

    let middleware = SwapMiddleware {
        swap_state: SwapState {
            rng,
            remote_comit_node_socket_addr,
            key_store,
            alice_actor_sender: Arc::new(Mutex::new(alice_actor_sender)),
        },
        client_factory: ClientFactory(client_factory),
        event_store: EventStore(event_store),
    };

    let (chain, pipelines) = single_pipeline(new_pipeline().add(middleware).build());

    build_router(chain, pipelines, |route| {
        route.post("/swaps").to(http_api::swap::post_swap::<C, E>);
        route
            .get("/swaps/:id")
            .with_path_extractor::<SwapId>()
            .to(http_api::swap::get_swap::<E>)
    })
}
