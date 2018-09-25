use bitcoin_rpc_client::TransactionId;
use failure::Error;
use ledger_query_service::{
    api::{LedgerQueryServiceApiClient, QueryId},
    bitcoin::BitcoinQuery,
};
use reqwest::{self, header::Location, Client, Url, UrlError};
use swap_protocols::ledger::bitcoin::Bitcoin;

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

#[derive(Fail, Debug)]
#[fail(display = "The provided endpoint was malformed.")]
pub struct MalformedEndpoint(#[cause] UrlError);

#[derive(Fail, Debug)]
#[fail(display = "The request failed to send.")]
pub struct FailedRequest(#[cause] reqwest::Error);

#[derive(Fail, Debug)]
#[fail(display = "The response did not contain a Location header.")]
pub struct MissingLocation;

#[derive(Fail, Debug)]
#[fail(display = "The returned URL could not be parsed.")]
pub struct MalformedLocation(#[cause] UrlError);

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for DefaultLedgerQueryServiceApiClient {
    fn create(&self, query: BitcoinQuery) -> Result<QueryId<Bitcoin>, Error> {
        let create_endpoint = self
            .endpoint
            .join("queries/bitcoin")
            .map_err(MalformedEndpoint)?;

        let response = self
            .client
            .post(create_endpoint)
            .json(&query)
            .send()
            .map_err(FailedRequest)?;

        let location = response
            .headers()
            .get::<Location>()
            .ok_or(MissingLocation)?;

        let url = Url::parse(location).map_err(MalformedLocation)?;

        Ok(QueryId::new(url))
    }

    fn fetch_results(&self, _query: &QueryId<Bitcoin>) -> Result<Vec<TransactionId>, Error> {
        unimplemented!()
    }

    fn delete(&self, _query: &QueryId<Bitcoin>) {
        unimplemented!()
    }
}
