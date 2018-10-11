use ledger_query_service::{Error, LedgerQueryServiceApiClient, QueryId};
use reqwest::{header::Location, Client, Url};
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

impl<L: Ledger, Q: Serialize> LedgerQueryServiceApiClient<L, Q>
    for DefaultLedgerQueryServiceApiClient
{
    fn create(&self, query: Q) -> Result<QueryId<L>, Error> {
        let type_id = &TypeId::of::<L>();
        let path = self.path.get(&type_id).ok_or(Error::UnsupportedLedger)?;

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

    fn fetch_results(&self, query: &QueryId<L>) -> Result<Vec<L::TxId>, Error> {
        let response = self
            .client
            .get(query.as_ref().clone())
            .send()
            .and_then(|mut res| res.json::<QueryResponse<L::TxId>>())
            .map_err(Error::FailedRequest)?;

        Ok(response.matching_transactions)
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
