use failure;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, Error, LedgerQueryServiceApiClient, QueryId,
};
use reqwest::{async::Client, header::LOCATION, Url};
use serde::{Deserialize, Serialize};
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use tokio::prelude::future::{self, Future};

#[derive(Debug)]
pub struct DefaultLedgerQueryServiceApiClient {
    client: Client,
    endpoint: Url,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse<T> {
    matching_transactions: Vec<T>,
}

impl DefaultLedgerQueryServiceApiClient {
    pub fn new(endpoint: Url) -> Self {
        DefaultLedgerQueryServiceApiClient {
            client: Client::new(),
            endpoint,
        }
    }

    fn _create<L: Ledger, Q: Serialize>(
        &self,
        path: &'static str,
        query: Q,
    ) -> Box<Future<Item = QueryId<L>, Error = Error> + Send> {
        let create_endpoint = match self.endpoint.join(path) {
            Ok(url) => url,
            Err(e) => return Box::new(future::err(Error::MalformedEndpoint(e))),
        };

        let query_id = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(Error::FailedRequest)
            .and_then(|response| {
                response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| Error::MalformedResponse(format_err!("missing location")))
                    .and_then(|value| {
                        value
                            .to_str()
                            .map_err(|e| Error::MalformedResponse(failure::Error::from(e)))
                    }).and_then(|location| {
                        Url::parse(location)
                            .map_err(|e| Error::MalformedResponse(failure::Error::from(e)))
                    })
            }).map(QueryId::new);

        Box::new(query_id)
    }

    fn _fetch_results<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<L::TxId>, Error = Error> + Send> {
        let transactions = self
            .client
            .get(query.as_ref().clone())
            .send()
            .and_then(|mut response| response.json::<QueryResponse<L::TxId>>())
            .map_err(Error::FailedRequest)
            .map(|response| response.matching_transactions);

        Box::new(transactions)
    }

    fn _delete<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<Future<Item = (), Error = Error> + Send> {
        Box::new(
            self.client
                .delete(query.as_ref().clone())
                .send()
                .map(|_| ())
                .map_err(Error::FailedRequest),
        )
    }
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(
        &self,
        query: BitcoinQuery,
    ) -> Box<Future<Item = QueryId<Bitcoin>, Error = Error> + Send> {
        self._create("queries/bitcoin", query)
    }

    fn fetch_results(
        &self,
        query: &QueryId<Bitcoin>,
    ) -> Box<Future<Item = Vec<<Bitcoin as Ledger>::TxId>, Error = Error> + Send> {
        self._fetch_results(query)
    }

    fn delete(&self, query: &QueryId<Bitcoin>) -> Box<Future<Item = (), Error = Error> + Send> {
        self._delete(&query)
    }
}

impl LedgerQueryServiceApiClient<Ethereum, EthereumQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(
        &self,
        query: EthereumQuery,
    ) -> Box<Future<Item = QueryId<Ethereum>, Error = Error> + Send> {
        self._create("queries/ethereum", query)
    }

    fn fetch_results(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<Future<Item = Vec<<Ethereum as Ledger>::TxId>, Error = Error> + Send> {
        self._fetch_results(query)
    }

    fn delete(&self, query: &QueryId<Ethereum>) -> Box<Future<Item = (), Error = Error> + Send> {
        self._delete(&query)
    }
}
