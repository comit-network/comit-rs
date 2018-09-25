use comit_client;
use event_store::{EventStore, InMemoryEventStore};
use gotham::{
    self,
    handler::HandlerFuture,
    middleware::{Middleware, NewMiddleware},
    pipeline::{new_pipeline, single::single_pipeline},
    router::{builder::*, Router},
    state::{State, StateData},
};
use http_api;
use rand::OsRng;
use std::{
    net::SocketAddr,
    ops::Deref,
    panic::RefUnwindSafe,
    sync::{Arc, Mutex},
};
use swaps::common::TradeId;

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct SwapId {
    pub id: TradeId,
}

#[derive(Clone, NewMiddleware)]
struct SwapMiddleware<F: Clone + StateData + Sync + RefUnwindSafe> {
    pub swap_state: SwapState,
    pub client_factory: F,
}

#[derive(StateData, Clone)]
pub struct SwapState {
    pub event_store: Arc<InMemoryEventStore<TradeId>>,
    pub rng: Arc<Mutex<OsRng>>,
    pub remote_comit_node_socket_addr: SocketAddr,
}

impl<C> Clone for ClientFactory<C> {
    fn clone(&self) -> Self {
        ClientFactory(self.0.clone())
    }
}

#[derive(StateData)]
pub struct ClientFactory<C: 'static>(pub Arc<comit_client::Factory<C>>);

impl<F: StateData + Clone + Sync + RefUnwindSafe> Middleware for SwapMiddleware<F> {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.swap_state);
        state.put(self.client_factory);
        chain(state)
    }
}

pub fn create_gotham_router<
    C: comit_client::Client + 'static,
    F: comit_client::Factory<C> + 'static,
>(
    event_store: Arc<InMemoryEventStore<TradeId>>,
    client_factory: Arc<F>,
    remote_comit_node_socket_addr: SocketAddr,
) -> Router {
    let rng = Arc::new(Mutex::new(
        OsRng::new().expect("Failed to get randomness from OS"),
    ));

    let middleware = SwapMiddleware {
        swap_state: SwapState {
            event_store,
            rng,
            remote_comit_node_socket_addr,
        },
        client_factory: ClientFactory(client_factory),
    };

    let (chain, pipelines) = single_pipeline(new_pipeline().add(middleware).build());

    build_router(chain, pipelines, |route| {
        route.post("/swap").to(http_api::swap::post_swap::<C>);
        route
            .get("/swap/:id")
            .with_path_extractor::<SwapId>()
            .to(http_api::swap::get_swap)
    })
}
