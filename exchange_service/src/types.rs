#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SecretHash(pub String); // string is hexadecimal!
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BtcBlockHeight(pub u32);
// TODO: implement Eth Web3 :)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EthAddress(pub String);
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EthTimestamp(pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Symbol(pub String); // Expected format: ETH-BTC or LTC-BTC

#[derive(Debug, Deserialize)]
pub struct Rate {
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Clone)]
pub struct TreasuryApiUrl(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: u32,
}
