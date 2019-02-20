use crate::btsieve::Error;
use futures::{future::Future, stream::Stream};
use std::time::{Duration, Instant};
use tokio::timer::Interval;

pub fn poll_until_item<
    I,
    Fut: Future<Item = Vec<I>, Error = Error> + Send + 'static,
    F: 'static + Send + FnMut() -> Fut,
>(
    poll_interval: Duration,
    mut f: F,
) -> Box<dyn Future<Item = I, Error = Error> + Send + 'static> {
    Box::new(
        Interval::new(Instant::now(), poll_interval)
            .map_err(|_| Error::Internal)
            .and_then(move |_| f())
            .filter_map(|mut items| {
                if items.is_empty() {
                    None
                } else {
                    Some(items.remove(0))
                }
            })
            .into_future()
            .map(|(item, _)| item.expect("Ticker shouldn't stop"))
            .map_err(|(e, _)| e),
    )
}
