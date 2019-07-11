use crate::{
    btsieve::{
        bitcoin::{BitcoinQuery, QueryBitcoin},
        ethereum::{EthereumQuery, QueryEthereum},
        poll_until_item::poll_until_item,
        Error, Query, QueryId,
    },
    swap_protocols::ledger::{Bitcoin, Ethereum, Ledger},
};
use core::time::Duration;
use futures::{stream::Stream, Async};
use reqwest::{header::LOCATION, r#async::Client, StatusCode, Url};
use serde::Deserialize;
use tokio::prelude::future::Future;

#[derive(Debug, Clone)]
pub struct BtsieveHttpClient {
    client: Client,
    endpoint: Url,
    create_bitcoin_transaction_query_endpoint: Url,
    create_bitcoin_block_query_endpoint: Url,
    create_ethereum_transaction_query_endpoint: Url,
    create_ethereum_block_query_endpoint: Url,
    create_ethereum_event_query_endpoint: Url,
    ethereum_poll_interval: Duration,
    bitcoin_poll_interval: Duration,
}

mod payloads {

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct TransactionId<T> {
        pub id: T,
    }

    #[derive(Debug, Deserialize)]
    pub struct Transaction<T> {
        pub transaction: T,
    }

    #[derive(Debug, Deserialize)]
    pub struct TransactionAndReceipt<T, R> {
        pub transaction: T,
        pub receipt: R,
    }
}

#[derive(Debug, Deserialize)]
struct QueryResponse<T> {
    matches: Vec<T>,
}

impl BtsieveHttpClient {
    pub fn new(
        endpoint: &Url,
        ethereum_poll_interval: Duration,
        ethereum_network: &str,
        bitcoin_poll_interval: Duration,
        bitcoin_network: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.clone(),
            create_bitcoin_transaction_query_endpoint: endpoint
                .join(format!("queries/bitcoin/{}/transactions", bitcoin_network).as_ref())
                .expect("invalid url"),
            create_bitcoin_block_query_endpoint: endpoint
                .join(format!("queries/bitcoin/{}/blocks", bitcoin_network).as_ref())
                .expect("invalid url"),
            create_ethereum_transaction_query_endpoint: endpoint
                .join(format!("queries/ethereum/{}/transactions", ethereum_network).as_ref())
                .expect("invalid url"),
            create_ethereum_block_query_endpoint: endpoint
                .join(format!("queries/ethereum/{}/blocks", ethereum_network).as_ref())
                .expect("invalid url"),
            create_ethereum_event_query_endpoint: endpoint
                .join(format!("queries/ethereum/{}/logs", ethereum_network).as_ref())
                .expect("invalid url"),
            ethereum_poll_interval,
            bitcoin_poll_interval,
        }
    }

    fn _create<L: Ledger, Q: Query>(
        &self,
        create_endpoint: Url,
        query: Q,
    ) -> Box<dyn Future<Item = QueryId<L>, Error = Error> + Send> {
        log::debug!("Creating {:?} at {}", query, create_endpoint);

        let endpoint = self.endpoint.clone();
        let query_id = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(move |e| {
                Error::FailedRequest(format!("Failed to create {:?} because {:?}", query, e))
            })
            .and_then(move |response| {
                if response.status() != StatusCode::CREATED {
                    if let Ok(Async::Ready(bytes)) = response.into_body().concat2().poll() {
                        log::error!(
                            "Failed to create query. btsieve returned: {}",
                            String::from_utf8(bytes.to_vec())
                                .expect("btsieve returned non-utf8 error")
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
                        endpoint.join(location).map_err(|e| {
                            Error::MalformedResponse(format!(
                                "Failed to parse {} as URL: {:?}",
                                location, e
                            ))
                        })
                    })
            })
            .inspect(|query_id| {
                log::info!("Created new query at location {}", query_id);
            })
            .map(QueryId::new);

        Box::new(query_id)
    }

    pub fn fetch_ids<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<dyn Future<Item = Vec<L::TxId>, Error = Error> + Send> {
        let url = query.as_ref().clone();
        let transactions = self
            .client
            .get(url.clone())
            .send()
            .and_then(|mut response| {
                response.json::<QueryResponse<payloads::TransactionId<L::TxId>>>()
            })
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to fetch results for {:?} because {:?}",
                    url, e
                ))
            })
            .map(|response| {
                response
                    .matches
                    .into_iter()
                    .map(|payload| payload.id)
                    .collect()
            });

        Box::new(transactions)
    }

    pub fn fetch_transactions<L: Ledger>(
        &self,
        query: &QueryId<L>,
    ) -> Box<dyn Future<Item = Vec<L::Transaction>, Error = Error> + Send> {
        let mut url = query.as_ref().clone();
        url.set_query(Some("return_as=transaction"));

        let transactions = self
            .client
            .get(url.clone())
            .send()
            .and_then(|mut response| {
                response.json::<QueryResponse<payloads::Transaction<L::Transaction>>>()
            })
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to fetch full results for {:?} because {:?}",
                    url, e
                ))
            })
            .map(|response| {
                response
                    .matches
                    .into_iter()
                    .map(|payload| payload.transaction)
                    .collect()
            });

        Box::new(transactions)
    }

    pub fn delete<L: Ledger>(
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

mod ethereum {
    use super::*;
    use ethereum_support::{Transaction, TransactionAndReceipt, TransactionReceipt, H256};

    impl QueryEthereum for BtsieveHttpClient {
        fn create(
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

        fn delete(
            &self,
            query: &QueryId<Ethereum>,
        ) -> Box<dyn Future<Item = (), Error = Error> + Send> {
            BtsieveHttpClient::delete(&self, query)
        }

        fn txid_results(
            &self,
            query: &QueryId<Ethereum>,
        ) -> Box<dyn Future<Item = Vec<H256>, Error = Error> + Send> {
            self.fetch_ids(&query)
        }
        fn transaction_results(
            &self,
            query: &QueryId<Ethereum>,
        ) -> Box<dyn Future<Item = Vec<Transaction>, Error = Error> + Send> {
            self.fetch_transactions(query)
        }

        fn transaction_first_result(
            &self,
            query: &QueryId<Ethereum>,
        ) -> Box<dyn Future<Item = Transaction, Error = Error> + Send> {
            let poll_client = self.clone();
            let query = query.clone();
            poll_until_item(self.ethereum_poll_interval, move || {
                poll_client.fetch_transactions(&query)
            })
        }

        fn transaction_and_receipt_first_result(
            &self,
            query: &QueryId<Ethereum>,
        ) -> Box<dyn Future<Item = TransactionAndReceipt, Error = Error> + Send> {
            let poll_client = self.client.clone();
            let query = query.clone();

            poll_until_item(self.ethereum_poll_interval, move || {
                let mut url = query.as_ref().clone();
                url.set_query(Some("return_as=transaction_and_receipt"));

                let results = poll_client
                    .get(url.clone())
                    .send()
                    .and_then(|mut response| {
                        response.json::<QueryResponse<
                            payloads::TransactionAndReceipt<Transaction, TransactionReceipt>,
                        >>()
                    })
                    .map_err(move |e| {
                        Error::FailedRequest(format!(
                            "Failed to fetch full results for {:?} because {:?}",
                            url, e
                        ))
                    })
                    .map(|response| {
                        response
                            .matches
                            .into_iter()
                            .map(|payload| TransactionAndReceipt {
                                transaction: payload.transaction,
                                receipt: payload.receipt,
                            })
                            .collect()
                    });

                Box::new(results)
            })
        }
    }

}

mod bitcoin {
    use super::*;
    use bitcoin_support::{Transaction, TransactionId};
    impl QueryBitcoin for BtsieveHttpClient {
        fn create(
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

        fn delete(
            &self,
            query: &QueryId<Bitcoin>,
        ) -> Box<dyn Future<Item = (), Error = Error> + Send> {
            BtsieveHttpClient::delete(&self, query)
        }

        fn txid_results(
            &self,
            query: &QueryId<Bitcoin>,
        ) -> Box<dyn Future<Item = Vec<TransactionId>, Error = Error> + Send> {
            self.fetch_ids(&query)
        }
        fn transaction_results(
            &self,
            query: &QueryId<Bitcoin>,
        ) -> Box<dyn Future<Item = Vec<Transaction>, Error = Error> + Send> {
            self.fetch_transactions(query)
        }
        fn transaction_first_result(
            &self,
            query: &QueryId<Bitcoin>,
        ) -> Box<dyn Future<Item = Transaction, Error = Error> + Send> {
            let poll_client = self.clone();
            let query = query.clone();
            poll_until_item(self.ethereum_poll_interval, move || {
                poll_client.fetch_transactions(&query)
            })
        }
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
