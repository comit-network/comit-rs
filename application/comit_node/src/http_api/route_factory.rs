use event_store;
use futures::sync::mpsc::UnboundedSender;
use http_api;
use std::{panic::RefUnwindSafe, sync::Arc};
use swap_protocols::{
    rfc003::{self, bob::PendingResponses, state_store},
    MetadataStore,
};
use swaps::common::SwapId;
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub fn create<
    E: event_store::EventStore<SwapId> + RefUnwindSafe,
    T: MetadataStore<SwapId>,
    S: state_store::StateStore<SwapId>,
>(
    event_store: Arc<E>,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)>,
) -> BoxedFilter<(impl Reply,)> {
    let path = warp::path(http_api::PATH);
    let rfc003 = warp::path(http_api::rfc003::swap::PATH);

    let event_store = warp::any().map(move || Arc::clone(&event_store));
    let metadata_store = warp::any().map(move || Arc::clone(&metadata_store));
    let state_store = warp::any().map(move || Arc::clone(&state_store));
    let sender = warp::any().map(move || sender.clone());
    let pending_responses = warp::any().map(move || Arc::clone(&pending_responses));

    let rfc003_post_swap = warp::post2()
        .and(warp::body::json())
        .and(sender)
        .and_then(http_api::rfc003::swap::post_swap);

    let rfc003_get_swap = warp::get2()
        .and(event_store.clone())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param::<SwapId>())
        .and_then(http_api::rfc003::swap::get_swap);

    let rfc003_post_action = warp::post2()
        .and(metadata_store)
        .and(pending_responses)
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<http_api::rfc003::action::Action>())
        .and(warp::body::json())
        .and_then(http_api::rfc003::action::post);

    path.and(rfc003_get_swap.or(rfc003.and(rfc003_post_action.or(rfc003_post_swap))))
        .recover(http_api::unpack_problem)
        .boxed()
}
