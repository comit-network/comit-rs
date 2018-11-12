use comit_client;
use event_store;
use futures::sync::mpsc::UnboundedSender;
use http_api;
use key_store::KeyStore;
use std::{
    net::SocketAddr,
    panic::RefUnwindSafe,
    sync::{Arc, Mutex},
};
use swap_protocols::{rfc003::state_store, MetadataStore};
use swaps::common::SwapId;
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Clone, Debug)]
pub struct SwapState {
    pub remote_comit_node_socket_addr: SocketAddr,
    pub key_store: Arc<KeyStore>,
    pub alice_actor_sender: Arc<Mutex<UnboundedSender<SwapId>>>,
}

pub fn create<
    C: comit_client::Client + 'static,
    F: comit_client::ClientFactory<C> + 'static,
    E: event_store::EventStore<SwapId> + RefUnwindSafe,
    T: MetadataStore<SwapId>,
    S: state_store::StateStore<SwapId>,
>(
    event_store: Arc<E>,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    client_factory: Arc<F>,
    remote_comit_node_socket_addr: SocketAddr,
    key_store: Arc<KeyStore>,
    alice_actor_sender: UnboundedSender<SwapId>,
) -> BoxedFilter<(impl Reply,)> {
    let path = warp::path(http_api::PATH);
    let rfc003 = warp::path(http_api::rfc003::swap::PATH);
    let swap_state = SwapState {
        remote_comit_node_socket_addr,
        key_store,
        alice_actor_sender: Arc::new(Mutex::new(alice_actor_sender)),
    };
    let swap_state = warp::any().map(move || swap_state.clone());

    let client_factory = warp::any().map(move || client_factory.clone());
    let event_store = warp::any().map(move || event_store.clone());
    let metadata_store = warp::any().map(move || metadata_store.clone());
    let state_store = warp::any().map(move || state_store.clone());

    let rfc003_post_swap = warp::post2()
        .and(swap_state)
        .and(client_factory)
        .and(event_store.clone())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::body::json())
        .and_then(http_api::rfc003::swap::post_swap);

    let rfc003_get_swap = warp::get2()
        .and(event_store)
        .and(metadata_store)
        .and(state_store)
        .and(warp::path::param())
        .and_then(http_api::rfc003::swap::get_swap);

    path.and(rfc003_get_swap.or(rfc003.and(rfc003_post_swap)))
        .recover(http_api::rfc003::swap::customize_error)
        .boxed()
}
