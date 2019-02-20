use futures::{
    future::Future,
    stream::{iter_ok, Stream},
};
use std::time::{Duration, Instant};
use tokio::timer::Interval;

pub fn poll_until_item<
    I,
    E,
    Fut: Future<Item = Vec<I>, Error = E> + Send + 'static,
    F: 'static + Send + FnMut() -> Fut,
>(
    poll_interval: Duration,
    mut f: F,
) -> Box<dyn Future<Item = I, Error = E> + Send + 'static> {
    let ticker = Interval::new(Instant::now(), poll_interval)
        .map_err(|e| unreachable!("Interval cannot error {:?}", e))
        .map(|_| ());

    Box::new(
        ticker
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
