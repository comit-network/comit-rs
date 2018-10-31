use failure;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, Error, LedgerQueryServiceApiClient, QueryId,
};
use reqwest::{async::Client, header::LOCATION, Url};
use serde::{Deserialize, Serialize};
use swap_protocols::ledger::{Bitcoin, Ethereum, Ledger};
use tokio::prelude::future::Future;

#[derive(Debug)]
pub struct DefaultLedgerQueryServiceApiClient {
    client: Client,
    create_bitcoin_transaction_query_endpoint: Url,
    create_bitcoin_block_query_endpoint: Url,
    create_ethereum_transaction_query_endpoint: Url,
    create_ethereum_block_query_endpoint: Url,
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
        }
    }

    fn _create<L: Ledger, Q: Serialize>(
        &self,
        create_endpoint: Url,
        query: &Q,
    ) -> Box<Future<Item = QueryId<L>, Error = Error> + Send> {
        let query_id = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(|e| Error::FailedRequest)
            .and_then(|response| {
                response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| Error::MalformedResponse)
                    //                    .ok_or_else(|| Error::MalformedResponse(format_err!("missing location")))
                    .and_then(|value| value.to_str().map_err(|e| Error::MalformedResponse))
                    .and_then(|location| Url::parse(location).map_err(|e| Error::MalformedResponse))
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
            .map_err(|e| Error::FailedRequest)
            .map(|response| response.matches);

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
                .map_err(|e| Error::FailedRequest),
        )
    }
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(
        &self,
        query: BitcoinQuery,
    ) -> Box<Future<Item = QueryId<Bitcoin>, Error = Error> + Send> {
        let endpoint = match &query {
            BitcoinQuery::Transaction { .. } => {
                self.create_bitcoin_transaction_query_endpoint.clone()
            }
            BitcoinQuery::Block { .. } => self.create_bitcoin_block_query_endpoint.clone(),
        };
        self._create(endpoint, &query)
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
        let endpoint = match &query {
            EthereumQuery::Transaction { .. } => {
                self.create_ethereum_transaction_query_endpoint.clone()
            }
            EthereumQuery::Block { .. } => self.create_ethereum_block_query_endpoint.clone(),
        };
        self._create(endpoint, &query)
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

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::TransactionId;
    use serde_json;

    #[test]
    fn json_deserialize() {
        let json = r#"{"query":{"to_address":"bcrt1qtfd0gvmdhx2uz267a8a3rpm4v55t8nuzgka2f5xzm4e06tg2d2dqxugdz7","confirmations_needed":1},"matches":["b29cb185d467b3a5faeb7a3f312175e336dbfcc8e9fecc8ad86e9106031315c2"]}"#;

        let blah: QueryResponse<TransactionId> = serde_json::from_str(json).unwrap();
    }
}
