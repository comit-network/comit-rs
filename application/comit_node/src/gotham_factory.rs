use comit_client;
use event_store::{EventStore, InMemoryEventStore};
use gotham::{
    self,
    handler::HandlerFuture,
    middleware::{Middleware, NewMiddleware},
    pipeline::{new_pipeline, single::single_pipeline},
    router::{builder::*, Router},
    state::State,
};
use http_api;
use rand::OsRng;
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use swaps::common::TradeId;

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct SwapId {
    pub id: TradeId,
}

#[derive(Clone, NewMiddleware)]
struct SwapMiddleware {
    pub swap_state: SwapState,
    //    pub client_factory: ClientFactory<C>,
}

#[derive(StateData, Clone)]
pub struct SwapState {
    pub event_store: Arc<InMemoryEventStore<TradeId>>,
    pub rng: Arc<Mutex<OsRng>>,
}

#[derive(StateData, Clone)]
pub struct ClientFactory<C: 'static>(pub Arc<comit_client::Factory<C>>);

impl<C> Deref for ClientFactory<C> {
    type Target = comit_client::Factory<C>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Middleware for SwapMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.swap_state);
        //        state.put(self.client_factory);
        chain(state)
    }
}

pub fn create_gotham_router<C: comit_client::Client + 'static>(
    event_store: Arc<InMemoryEventStore<TradeId>>,
    //    client_factory: comit_client::Factory<C>,
) -> Router {
    let rng = Arc::new(Mutex::new(
        OsRng::new().expect("Failed to get randomness from OS"),
    ));
    let middleware = SwapMiddleware {
        swap_state: SwapState { event_store, rng },
        //        client_factory: ClientFactory(Arc::new(client_factory)),
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
