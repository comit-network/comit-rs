use ledger_query_service::api::{LedgerQueryServiceApiClient, Query};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{prelude::*, timer::Delay};

#[derive(Clone)]
pub struct AsyncLedgerQueryService {
    inner: Arc<LedgerQueryServiceApiClient>,
    poll_interval: Duration,
}

impl AsyncLedgerQueryService {
    pub fn new(inner: Arc<LedgerQueryServiceApiClient>, poll_interval: Duration) -> Self {
        AsyncLedgerQueryService {
            inner,
            poll_interval,
        }
    }

    pub fn fetch_results(&self, query: Query) -> FetchResultsFuture {
        FetchResultsFuture::new(self.inner.clone(), query, self.poll_interval)
    }
}

pub struct FetchResultsFuture {
    api_client: Arc<LedgerQueryServiceApiClient>,
    query: Query,
    poll_interval: Duration,
    next_try: Delay,
}

impl FetchResultsFuture {
    fn new(
        api_client: Arc<LedgerQueryServiceApiClient>,
        query: Query,
        poll_interval: Duration,
    ) -> Self {
        FetchResultsFuture {
            api_client,
            query,
            poll_interval,
            next_try: Self::compute_next_try(poll_interval),
        }
    }

    fn compute_next_try(interval: Duration) -> Delay {
        Delay::new(Instant::now() + interval)
    }
}

impl Future for FetchResultsFuture {
    type Item = Vec<String>;
    type Error = ();

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        let _ = try_ready!(self.next_try.poll().map_err(|_| ()));

        match self.api_client.fetch_query_results(&self.query) {
            Ok(ref results) if results.len() > 0 => Ok(Async::Ready(results.clone())),
            _ => {
                self.next_try = Self::compute_next_try(self.poll_interval);
                self.poll()
            }
        }
    }
}
