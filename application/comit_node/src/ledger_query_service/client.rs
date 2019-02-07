use crate::{
    ledger_query_service::{
        bitcoin::BitcoinQuery, ethereum::EthereumQuery, CreateQuery, Error, FetchFullQueryResults,
        FetchQueryResults, LedgerQueryServiceApiClient, Query, QueryId,
    },
    swap_protocols::ledger::{Bitcoin, Ethereum, Ledger},
};
use futures::{stream::Stream, Async};
use reqwest::{header::LOCATION, r#async::Client, StatusCode, Url};
use serde::Deserialize;
use tokio::prelude::future::Future;

#[derive(Debug)]
pub struct DefaultLedgerQueryServiceApiClient {
    client: Client,
    create_bitcoin_transaction_query_endpoint: Url,
    create_bitcoin_block_query_endpoint: Url,
    create_ethereum_transaction_query_endpoint: Url,
    create_ethereum_block_query_endpoint: Url,
    create_ethereum_event_query_endpoint: Url,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse<T> {
    matches: Vec<T>,
}

impl DefaultLedgerQueryServiceApiClient {
    pub fn new(endpoint: &Url) -> Self {
        DefaultLedgerQueryServiceApiClient {
            client: Client::new(),
            create_bitcoin_transaction_query_endpoint: endpoint
                .join("queries/bitcoin/transactions")
                .expect("invalid url"),
            create_bitcoin_block_query_endpoint: endpoint
                .join("queries/bitcoin/blocks")
                .expect("invalid url"),
            create_ethereum_transaction_query_endpoint: endpoint
                .join("queries/ethereum/transactions")
                .expect("invalid url"),
            create_ethereum_block_query_endpoint: endpoint
                .join("queries/ethereum/blocks")
                .expect("invalid url"),
            create_ethereum_event_query_endpoint: endpoint
                .join("queries/ethereum/logs")
                .expect("invalid url"),
        }
    }

    fn _create<L: Ledger, Q: Query>(
        &self,
        create_endpoint: Url,
        query: Q,
    ) -> Box<dyn Future<Item = QueryId<L>, Error = Error> + Send> {
        debug!("Creating {:?} at {}", query, create_endpoint);

        let query_id = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(move |e| {
                Error::FailedRequest(format!("Failed to create {:?} because {:?}", query, e))
            })
            .and_then(|response| {
                if response.status() != StatusCode::CREATED {
                    if let Ok(Async::Ready(bytes)) = response.into_body().concat2().poll() {
                        error!(
                            "Failed to create query. LQS returned: {}",
                            String::from_utf8(bytes.to_vec()).expect("LQS returned non-utf8 error")
                        );
                    }

                    return Err(Error::MalformedResponse(
                        "Could not create query".to_string(),
                    ));
                }

                response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| {
                        Error::MalformedResponse(
                            "Location header was not present in response".to_string(),
                        )
                    })
                    .and_then(|value| {
                        value.to_str().map_err(|e| {
                            Error::MalformedResponse(format!(
                                "Unable to extract Location from response: {:?}",
                                e
                            ))
                        })
                    })
                    .and_then(|location| {
                        Url::parse(location).map_err(|e| {
                            Error::MalformedResponse(format!(
                                "Failed to parse {} as URL: {:?}",
                                location, e
                            ))
                        })
                    })
            })
            .inspect(|query_id| {
                info!("Created new query at location {}", query_id);
            })
            .map(QueryId::new);

        Box::new(query_id)
    }

    fn fetch_results<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<dyn Future<Item = Vec<L::TxId>, Error = Error> + Send> {
        let url = query.as_ref().clone();
        let transactions = self
            .client
            .get(url.clone())
            .send()
            .and_then(|mut response| response.json::<QueryResponse<L::TxId>>())
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to fetch results for {:?} because {:?}",
                    url, e
                ))
            })
            .map(|response| response.matches);

        Box::new(transactions)
    }

    fn fetch_full_results<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<dyn Future<Item = Vec<L::Transaction>, Error = Error> + Send> {
        let mut url = query.as_ref().clone();
        url.set_query(Some("expand_results=true"));

        let transactions = self
            .client
            .get(url.clone())
            .send()
            .and_then(|mut response| response.json::<QueryResponse<L::Transaction>>())
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to fetch full results for {:?} because {:?}",
                    url, e
                ))
            })
            .map(|response| response.matches);

        Box::new(transactions)
    }

    fn _delete<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<dyn Future<Item = (), Error = Error> + Send> {
        Box::new(
            self.client
                .delete(query.as_ref().clone())
                .send()
                .map(|_| ())
                .map_err(|e| {
                    Error::FailedRequest(format!("Failed to delete query because {:?}", e))
                }),
        )
    }
}

impl CreateQuery<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn create_query(
        &self,
        query: BitcoinQuery,
    ) -> Box<dyn Future<Item = QueryId<Bitcoin>, Error = Error> + Send> {
        let endpoint = match &query {
            BitcoinQuery::Transaction { .. } => {
                self.create_bitcoin_transaction_query_endpoint.clone()
            }
            BitcoinQuery::Block { .. } => self.create_bitcoin_block_query_endpoint.clone(),
        };
        self._create(endpoint, query)
    }
}

impl FetchQueryResults<Bitcoin> for DefaultLedgerQueryServiceApiClient {
    fn fetch_query_results(
        &self,
        query: &QueryId<Bitcoin>,
    ) -> Box<dyn Future<Item = Vec<<Bitcoin as Ledger>::TxId>, Error = Error> + Send> {
        self.fetch_results(query)
    }
}

impl FetchFullQueryResults<Bitcoin> for DefaultLedgerQueryServiceApiClient {
    fn fetch_full_query_results(
        &self,
        query: &QueryId<Bitcoin>,
    ) -> Box<dyn Future<Item = Vec<<Bitcoin as Ledger>::Transaction>, Error = Error> + Send> {
        self.fetch_full_results(query)
    }
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn delete(&self, query: &QueryId<Bitcoin>) -> Box<dyn Future<Item = (), Error = Error> + Send> {
        self._delete(&query)
    }
}

impl CreateQuery<Ethereum, EthereumQuery> for DefaultLedgerQueryServiceApiClient {
    fn create_query(
        &self,
        query: EthereumQuery,
    ) -> Box<dyn Future<Item = QueryId<Ethereum>, Error = Error> + Send> {
        let endpoint = match &query {
            EthereumQuery::Transaction { .. } => {
                self.create_ethereum_transaction_query_endpoint.clone()
            }
            EthereumQuery::Block { .. } => self.create_ethereum_block_query_endpoint.clone(),
            EthereumQuery::Event { .. } => self.create_ethereum_event_query_endpoint.clone(),
        };
        self._create(endpoint, query)
    }
}

impl FetchQueryResults<Ethereum> for DefaultLedgerQueryServiceApiClient {
    fn fetch_query_results(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = Vec<<Ethereum as Ledger>::TxId>, Error = Error> + Send> {
        self.fetch_results(query)
    }
}

impl FetchFullQueryResults<Ethereum> for DefaultLedgerQueryServiceApiClient {
    fn fetch_full_query_results(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = Vec<<Ethereum as Ledger>::Transaction>, Error = Error> + Send> {
        self.fetch_full_results(query)
    }
}

impl LedgerQueryServiceApiClient<Ethereum, EthereumQuery> for DefaultLedgerQueryServiceApiClient {
    fn delete(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = (), Error = Error> + Send> {
        self._delete(&query)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::TransactionId;

    #[test]
    fn json_deserialize() {
        let json =
            r#"{"matches":["b29cb185d467b3a5faeb7a3f312175e336dbfcc8e9fecc8ad86e9106031315c2"]}"#;

        let _: QueryResponse<TransactionId> = serde_json::from_str(json).unwrap();
    }
}
