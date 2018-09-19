use futures_ext::PollingFuture;
use ledger_query_service::api::{LedgerQueryServiceApiClient, Query};
use std::{sync::Arc, time::Duration};
use tokio::prelude::*;

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

    pub fn fetch_results(&self, query: Query) -> PollingFuture<FetchResultsFuture> {
        PollingFuture::new(
            FetchResultsFuture {
                api_client: self.inner.clone(),
                query,
            },
            self.poll_interval,
        )
    }
}

pub struct FetchResultsFuture {
    api_client: Arc<LedgerQueryServiceApiClient>,
    query: Query,
}

impl Future for FetchResultsFuture {
    type Item = Vec<String>;
    type Error = ();

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        match self.api_client.fetch_query_results(&self.query) {
            Ok(ref results) if results.len() > 0 => Ok(Async::Ready(results.clone())),
            Ok(_) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
