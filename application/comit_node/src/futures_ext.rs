use std::time::Duration;
use tokio::prelude::*;
use tokio_timer::{self, Delay};

pub trait FutureTemplate<D> {
    type Future: Future + Sized;

    fn into_future(self, dependencies: D) -> Self::Future;
}

pub trait StreamTemplate<D> {
    type Stream: Stream + Sized;

    fn into_stream(self, dependencies: D) -> Self::Stream;
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

    pub fn create_stream_from_template<T: StreamTemplate<D>>(
        &self,
        stream_template: T,
    ) -> <T as StreamTemplate<D>>::Stream {
        stream_template.into_stream(self.dependencies.clone())
    }
}

/// A future that polls the inner future in the given interval until it returns Ready
pub struct PollUntilReady<F> {
    inner: F,
    poll_interval: Duration,
    next_try: Delay,
}

impl<F> PollUntilReady<F> {
    pub fn new(inner: F, poll_interval: Duration) -> Self {
        PollUntilReady {
            inner,
            poll_interval,
            next_try: Self::compute_next_try(poll_interval),
        }
    }

    fn compute_next_try(interval: Duration) -> Delay {
        tokio_timer::sleep(interval)
    }
}

impl<F: Future> Future for PollUntilReady<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        let _ = try_ready!(
            self.next_try
                .poll()
                .map_err(|e| panic!("Unable to poll timer of PollUntilReady: {:?}", e))
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
