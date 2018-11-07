use comit_client;
use event_store;
use futures::sync::mpsc::UnboundedSender;
use http_api;
use key_store::KeyStore;
use rand::OsRng;
use std::{
    net::SocketAddr,
    panic::RefUnwindSafe,
    sync::{Arc, Mutex},
};
use swap_metadata_store;
use swap_protocols::rfc003::state_store;
use swaps::common::TradeId;
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Clone, Debug)]
pub struct SwapState {
    pub rng: Arc<Mutex<OsRng>>,
    pub remote_comit_node_socket_addr: SocketAddr,
    pub key_store: Arc<KeyStore>,
    pub alice_actor_sender: Arc<Mutex<UnboundedSender<TradeId>>>,
}

pub fn create<
    C: comit_client::Client + 'static,
    F: comit_client::ClientFactory<C> + 'static,
    E: event_store::EventStore<TradeId> + RefUnwindSafe,
    T: swap_metadata_store::SwapMetadataStore<TradeId>,
    S: state_store::StateStore<TradeId>,
>(
    event_store: Arc<E>,
    swap_metadata_store: Arc<T>,
    state_store: Arc<S>,
    client_factory: Arc<F>,
    remote_comit_node_socket_addr: SocketAddr,
    key_store: Arc<KeyStore>,
    alice_actor_sender: UnboundedSender<TradeId>,
) -> BoxedFilter<(impl Reply,)> {
    let path = warp::path(http_api::swap::PATH);

    let rng = Arc::new(Mutex::new(
        OsRng::new().expect("Failed to get randomness from OS"),
    ));
    let swap_state = SwapState {
        rng,
        remote_comit_node_socket_addr,
        key_store,
        alice_actor_sender: Arc::new(Mutex::new(alice_actor_sender)),
    };
    let swap_state = warp::any().map(move || swap_state.clone());

    let client_factory = warp::any().map(move || client_factory.clone());
    let event_store = warp::any().map(move || event_store.clone());
    let swap_metadata_store = warp::any().map(move || swap_metadata_store.clone());
    let state_store = warp::any().map(move || state_store.clone());

    let post_swap = warp::post2()
        .and(swap_state)
        .and(client_factory)
        .and(event_store.clone())
        .and(swap_metadata_store.clone())
        .and(state_store.clone())
        .and(warp::body::json())
        .and_then(http_api::swap::post_swap);

    let get_swap = warp::get2()
        .and(event_store)
        .and(swap_metadata_store)
        .and(state_store)
        .and(warp::path::param())
        .and_then(http_api::swap::get_swap);

    path.and(post_swap.or(get_swap))
        .recover(http_api::swap::customize_error)
        .boxed()
}
