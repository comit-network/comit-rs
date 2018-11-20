use futures::sync::mpsc::UnboundedSender;
use http_api;
use std::sync::Arc;
use swap_protocols::{
    rfc003::{self, state_store},
    MetadataStore,
};
use swaps::common::SwapId;
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub fn create<T: MetadataStore<SwapId>, S: state_store::StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)>,
) -> BoxedFilter<(impl Reply,)> {
    let path = warp::path(http_api::PATH);
    let rfc003 = warp::path(http_api::rfc003::swap::PROTOCOL_NAME);

    let metadata_store = warp::any().map(move || metadata_store.clone());
    let state_store = warp::any().map(move || state_store.clone());
    let sender = warp::any().map(move || sender.clone());

    let rfc003_post_swap = warp::post2()
        .and(warp::body::json())
        .and(sender)
        .and_then(http_api::rfc003::swap::post_swap);

    let rfc003_get_swap = warp::get2()
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param())
        .and_then(http_api::rfc003::swap::get_swap);

    let rfc003_get_swaps = warp::get2()
        .and(metadata_store)
        .and(state_store)
        .and_then(http_api::rfc003::swap::get_swaps);

    path.and(
        rfc003_get_swap
            .or(rfc003_get_swaps)
            .or(rfc003.and(rfc003_post_swap)),
    )
    .recover(http_api::rfc003::swap::customize_error)
    .boxed()
}
