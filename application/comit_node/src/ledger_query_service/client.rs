use bitcoin_rpc_client::TransactionId;
use ethereum_support::web3::types::H256;
use failure::Error as FailureError;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, LedgerQueryServiceApiClient, QueryId,
};
use reqwest::{self, header::Location, Client, Url, UrlError};
use serde::Deserialize;
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};

#[derive(Debug)]
pub struct DefaultLedgerQueryServiceApiClient {
    client: Client,
    endpoint: Url,
}

impl DefaultLedgerQueryServiceApiClient {
    pub fn new(endpoint: Url) -> Self {
        DefaultLedgerQueryServiceApiClient {
            client: Client::new(),
            endpoint,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse<T> {
    matching_transactions: Vec<T>,
}

#[derive(Fail, Debug)]
enum Error {
    #[fail(display = "The provided endpoint was malformed.")]
    MalformedEndpoint(UrlError),
    #[fail(display = "The request failed to send.")]
    FailedRequest(reqwest::Error),
    #[fail(display = "The response did not contain a Location header.")]
    MissingLocation,
    #[fail(display = "The returned URL could not be parsed.")]
    MalformedLocation(#[cause] UrlError),
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(&self, query: BitcoinQuery) -> Result<QueryId<Bitcoin>, FailureError> {
        let create_endpoint = self
            .endpoint
            .join("queries/bitcoin")
            .map_err(Error::MalformedEndpoint)?;

        let response = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(Error::FailedRequest)?;

        let location = response
            .headers()
            .get::<Location>()
            .ok_or(Error::MissingLocation)?;

        let url = Url::parse(location).map_err(Error::MalformedLocation)?;

        Ok(QueryId::new(url))
    }

    fn fetch_results(&self, query: &QueryId<Bitcoin>) -> Result<Vec<TransactionId>, FailureError> {
        let mut response = self
            .client
            .get(query.as_ref().clone())
            .send()
            .map_err(Error::FailedRequest)?;

        Ok(response
            .json::<QueryResponse<TransactionId>>()?
            .matching_transactions)
    }

    fn delete(&self, query: &QueryId<Bitcoin>) {
        let response = self.client.delete(query.as_ref().clone()).send();

        if let Err(e) = response {
            error!(
                "Could not delete query {:?} on ledger_query_service: {}",
                query, e
            );
        };
    }
}

impl LedgerQueryServiceApiClient<Ethereum, EthereumQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(&self, query: EthereumQuery) -> Result<QueryId<Ethereum>, FailureError> {
        let create_endpoint = self
            .endpoint
            .join("queries/ethereum")
            .map_err(Error::MalformedEndpoint)?;

        let response = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(Error::FailedRequest)?;

        let location = response
            .headers()
            .get::<Location>()
            .ok_or(Error::MissingLocation)?;

        let url = Url::parse(location).map_err(Error::MalformedLocation)?;

        Ok(QueryId::new(url))
    }
    fn fetch_results(&self, query: &QueryId<Ethereum>) -> Result<Vec<H256>, FailureError> {
        let mut response = self
            .client
            .get(query.as_ref().clone())
            .send()
            .map_err(Error::FailedRequest)?;

        Ok(response
            .json::<QueryResponse<H256>>()?
            .matching_transactions)
    }
    fn delete(&self, query: &QueryId<Ethereum>) {
        let response = self.client.delete(query.as_ref().clone()).send();

        if let Err(e) = response {
            error!(
                "Could not delete query {:?} on ledger_query_service: {}",
                query, e
            );
        };
    }
}
