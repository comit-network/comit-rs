use failure;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, Error, LedgerQueryServiceApiClient, QueryId,
};
use reqwest::{async::Client, header::LOCATION, Url};
use serde::{Deserialize, Serialize};
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
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
pub struct TransactionQueryResponse<T> {
    matching_transactions: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct BlockQueryResponse<B> {
    matching_blocks: Vec<B>,
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
            .and_then(|mut response| response.json::<TransactionQueryResponse<L::TxId>>())
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
