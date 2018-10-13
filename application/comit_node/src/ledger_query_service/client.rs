use failure;
use ledger_query_service::{Error, LedgerQueryServiceApiClient, QueryId};
use reqwest::{async::Client, header::LOCATION, Url};
use serde::{Deserialize, Serialize};
use std::{any::TypeId, collections::HashMap};
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use tokio::prelude::future::{self, Future};

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
    fn create(&self, query: Q) -> Box<Future<Item = QueryId<L>, Error = Error> + Send> {
        let type_id = &TypeId::of::<L>();

        let create_endpoint = match self.path.get(&type_id).map(|path| self.endpoint.join(path)) {
            Some(Ok(url)) => url,
            Some(Err(e)) => return Box::new(future::err(Error::MalformedEndpoint(e))),
            None => return Box::new(future::err(Error::UnsupportedLedger)),
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

    fn fetch_results(
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

    fn delete(&self, query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send> {
        Box::new(
            self.client
                .delete(query.as_ref().clone())
                .send()
                .map(|_| ())
                .map_err(Error::FailedRequest),
        )
    }
}
