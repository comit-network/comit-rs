use reqwest;
use std::{collections::HashMap, fmt, str::FromStr};
use uuid::{ParseError, Uuid};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl FromStr for TradeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let uid = Uuid::from_str(s)?;
        Ok(TradeId(uid))
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct ComitNodeApiUrl(pub String);

#[allow(dead_code)]
#[derive(Debug)]
pub struct DefaultApiClient {
    pub url: ComitNodeApiUrl,
    pub client: reqwest::Client,
}

pub trait ApiClient {
    fn send_swap_request(&self, _: SwapRequest) -> Result<SwapCreated, reqwest::Error>;
    fn get_swap_status(&self, id: TradeId) -> Result<SwapStatus, reqwest::Error>;
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(tag = "status")]
pub enum SwapStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted { funding_required: String },
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "redeemable")]
    Redeemable {
        contract_address: String,
        data: String,
        gas: u64,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct SwapCreated {
    pub id: TradeId,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Ledger {
    pub value: String,
    pub identity: String,
    #[serde(flatten)]
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Asset {
    pub value: String,
    #[serde(flatten)]
    pub parameters: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SwapRequest {
    pub alpha_ledger: Ledger,
    pub beta_ledger: Ledger,
    pub alpha_asset: Asset,
    pub beta_asset: Asset,
}

impl ApiClient for DefaultApiClient {
    fn send_swap_request(&self, swap_request: SwapRequest) -> Result<SwapCreated, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/swaps/rfc003", self.url.0).as_str())
            .json(&swap_request)
            .send()
            .and_then(|mut res| res.json::<SwapCreated>())
    }

    fn get_swap_status(&self, id: TradeId) -> Result<SwapStatus, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/swaps/{}", self.url.0, id).as_str())
            .send()
            .and_then(|mut res| res.json::<SwapStatus>())
    }
}
