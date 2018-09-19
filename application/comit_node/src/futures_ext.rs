use std::time::{Duration, Instant};
use tokio::{prelude::*, timer::Delay};

pub trait FutureTemplate<D> {
    type Future: Future + Sized;

    fn into_future(self, dependencies: D) -> Self::Future;
}

pub struct FutureFactory<D> {
    dependencies: D,
}

impl<D: Clone> FutureFactory<D> {
    pub fn new(dependencies: D) -> Self {
        FutureFactory { dependencies }
    }

    pub fn create_future_from_template<T: FutureTemplate<D>>(
        &self,
        future_template: T,
    ) -> <T as FutureTemplate<D>>::Future {
        future_template.into_future(self.dependencies.clone())
    }
}

/// A future that polls the inner future in the given interval until it returns Ready
pub struct PollingFuture<F> {
    inner: F,
    poll_interval: Duration,
    next_try: Delay,
}

impl<F> PollingFuture<F> {
    pub fn new(inner: F, poll_interval: Duration) -> Self {
        PollingFuture {
            inner,
            poll_interval,
            next_try: Self::compute_next_try(poll_interval),
        }
    }

    fn compute_next_try(interval: Duration) -> Delay {
        Delay::new(Instant::now() + interval)
    }
}

impl<F: Future> Future for PollingFuture<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        let _ = try_ready!(
            self.next_try
                .poll()
                .map_err(|_| panic!("Unable to poll timer of PollingFuture"))
        );

        let inner = self.inner.poll();

        match inner {
            Ok(Async::NotReady) => {
                self.next_try = Self::compute_next_try(self.poll_interval);
                self.poll()
            }
            _ => inner,
        }
    }
}
