use futures::{
    future::Future,
    stream::{iter_ok, Stream},
};
use std::time::{Duration, Instant};
use tokio::timer::Interval;

#[allow(dead_code)]
fn poll_dedup<
    I: PartialEq + Clone + 'static + Send,
    E: Send + 'static,
    Fut: Future<Item = Vec<I>, Error = E> + Send + 'static,
    F: 'static + Send + FnMut() -> Fut,
    Ticker: Stream<Item = (), Error = ()> + Send + 'static,
>(
    ticker: Ticker,
    mut f: F,
) -> Box<dyn Stream<Item = I, Error = E> + Send + 'static> {
    let mut seen = vec![];

    Box::new(
        ticker
            .map_err(|_| unreachable!("Ticker cannot error"))
            .and_then(move |_| f())
            .map(iter_ok)
            .flatten()
            .filter(move |item| {
                let is_new = !seen.contains(item);

                if is_new {
                    seen.push(item.clone());
                }
                is_new
            }),
    )
}

/// Keep polling a future that returns a vec and puts the results
/// deduplicated results into a stream.
// Here in case we want to use it later
#[allow(dead_code)]
pub fn poll_future_into_stream<
    I: PartialEq + Clone + 'static + Send,
    E: Send + 'static,
    Fut: Future<Item = Vec<I>, Error = E> + Send + 'static,
    F: 'static + Send + FnMut() -> Fut,
>(
    poll_interval: Duration,
    f: F,
) -> Box<dyn Stream<Item = I, Error = E> + Send + 'static> {
    let ticker = Interval::new(Instant::now(), poll_interval)
        .map_err(|e| unreachable!("Interval cannot error {:?}", e))
        .map(|_| ());

    poll_dedup(ticker, f)
}

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
            .map_err(|_| unreachable!("Ticker cannot panic"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future, sync::mpsc};
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };
    use tokio::{prelude::future::Either, runtime::Runtime, timer::Delay};

    fn init_test(
        results: &Arc<Mutex<Option<Vec<u32>>>>,
    ) -> (
        Arc<Mutex<u32>>,
        mpsc::UnboundedSender<()>,
        Box<dyn Stream<Item = u32, Error = ()> + Send + 'static>,
    ) {
        let (sender, receiver) = mpsc::unbounded();
        let number_of_invocations = Arc::new(Mutex::new(0u32));
        let results = Arc::clone(&results);

        (
            Arc::clone(&number_of_invocations),
            sender,
            Box::new(poll_dedup(receiver, move || {
                *number_of_invocations.lock().unwrap() += 1;
                match results.lock().unwrap().take() {
                    Some(results) => future::ok(results),
                    None => future::ok(vec![]),
                }
            })),
        )
    }

    #[test]
    fn should_emit_transactions_as_they_appear_without_waiting_for_the_next_tick() {
        let _ = pretty_env_logger::try_init();

        let mut runtime = Runtime::new().unwrap();

        let next_result = Arc::new(Mutex::new(Some(vec![1, 2, 3])));

        let (number_of_invocations, sender, stream) = init_test(&next_result);

        sender.unbounded_send(()).unwrap();
        let (result, stream) = runtime
            .block_on(stream.into_future().map_err(|_| unreachable!()))
            .unwrap();

        assert_eq!(result, Some(1));

        let (result, stream) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();
        assert_eq!(result, Some(2));

        let (result, _) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();
        assert_eq!(result, Some(3));
        assert_eq!(
            *number_of_invocations.lock().unwrap(),
            1,
            "should receive all three results within a single poll"
        );
    }

    #[test]
    fn should_not_emit_same_transaction_twice() {
        let _ = pretty_env_logger::try_init();

        let mut runtime = Runtime::new().unwrap();

        let next_result = Arc::new(Mutex::new(Some(vec![1])));
        let (number_of_invocations, sender, stream) = init_test(&next_result);

        sender.unbounded_send(()).unwrap();
        let (result, stream) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();

        assert_eq!(result, Some(1));

        *next_result.lock().unwrap() = Some(vec![1, 2]);

        sender.unbounded_send(()).unwrap();
        let (result, _) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();

        assert_eq!(result, Some(2));

        assert_eq!(
            *number_of_invocations.lock().unwrap(),
            2,
            "should have polled twice"
        );
    }

    #[test]
    fn given_no_results_should_not_emit_anything() {
        let _ = pretty_env_logger::try_init();
        let mut runtime = Runtime::new().unwrap();
        let next_result = Arc::new(Mutex::new(Some(vec![])));
        let (_number_of_invocations, sender, stream) = init_test(&next_result);
        sender.unbounded_send(()).unwrap();

        let either = runtime
            .block_on(
                stream
                    .into_future()
                    .select2(Delay::new(Instant::now() + Duration::from_secs(1))),
            )
            .map_err(|_| ())
            .unwrap();

        // A stream of no items will never complete.
        // Thus we `select2` it with a delay that completes after 1 second
        // We have to do this weird assertion because some things are not Debug :(
        // TL;DR: If we don't hit this branch, the Either is Either::B (the timeout) so
        // we are fine.
        if let Either::A(_transaction) = either {
            panic!("should not emit a transaction")
        }
    }
}
