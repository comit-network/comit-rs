use failure::Error as FailureError;
use ledger_query_service::{LedgerQueryServiceApiClient, QueryId};
use reqwest::{self, header::Location, Client, Url, UrlError};
use serde::{Deserialize, Serialize};
use std::{any::TypeId, collections::HashMap};
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};

#[derive(Debug)]
pub struct DefaultLedgerQueryServiceApiClient {
    client: Client,
    endpoint: Url,
    path: HashMap<TypeId, &'static str>,
}

impl DefaultLedgerQueryServiceApiClient {
    pub fn new(endpoint: Url) -> Self {
        DefaultLedgerQueryServiceApiClient {
            client: Client::new(),
            endpoint,
            path: hashmap!(TypeId::of::<Bitcoin>() => "queries/bitcoin", TypeId::of::<Ethereum>() => "queries/ethereum"),
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
    #[fail(display = "The ledger is not support.")]
    UnsupportedLedger(),
}

impl<L: Ledger, Q: Serialize> LedgerQueryServiceApiClient<L, Q>
    for DefaultLedgerQueryServiceApiClient
{
    fn create(&self, query: Q) -> Result<QueryId<L>, FailureError> {
        let type_id = &TypeId::of::<L>();
        let path = self
            .path
            .get(&type_id)
            .ok_or_else(Error::UnsupportedLedger)?;

        let create_endpoint = self.endpoint.join(path).map_err(Error::MalformedEndpoint)?;

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

    fn fetch_results(&self, query: &QueryId<L>) -> Result<Vec<L::TxId>, FailureError> {
        let mut response = self
            .client
            .get(query.as_ref().clone())
            .send()
            .map_err(Error::FailedRequest)?;

        Ok(response
            .json::<QueryResponse<L::TxId>>()?
            .matching_transactions)
    }

    fn delete(&self, query: &QueryId<L>) {
        let response = self.client.delete(query.as_ref().clone()).send();

        if let Err(e) = response {
            error!(
                "Could not delete query {:?} on ledger_query_service: {}",
                query, e
            );
        };
    }
}
