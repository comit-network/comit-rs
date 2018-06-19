use serde::Deserialize;
use serde::Deserializer;
use serde::de;
use serde::de::Visitor;
use std::fmt;
use std::str::FromStr;
use trading_service_api_client::ApiClient;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::TradingApiUrl;
use trading_service_api_client::TradingServiceError;
use trading_service_api_client::create_client;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Currency(String);

impl fmt::Display for Currency {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

impl From<String> for Currency {
    //TODO: validate format
    fn from(s: String) -> Self {
        Currency(s)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Symbol {
    traded: Currency,
    base: Currency,
}

#[derive(Debug)]
pub enum ParseSymbolErr {
    BadFormat,
}

impl Symbol {
    pub fn get_base_currency(&self) -> &Currency {
        &self.base
    }
    pub fn get_traded_currency(&self) -> &Currency {
        &self.traded
    }
}

impl FromStr for Symbol {
    type Err = ParseSymbolErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let currencies: Vec<&str> = s.split("-").collect();

        if currencies.len() != 2 {
            return Err(ParseSymbolErr::BadFormat);
        }

        let traded = Currency::from(currencies[0].to_string());
        let base = Currency::from(currencies[1].to_string());

        Ok(Symbol { traded, base })
    }
}
impl fmt::Display for Symbol {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}-{}", self.traded, self.base)
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Symbol, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(SymbolVisitor)
    }
}

struct SymbolVisitor;

impl<'de> Visitor<'de> for SymbolVisitor {
    type Value = Symbol;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a symbol (ETH-BTC)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Symbol::from_str(value)
            .map_err(|error| E::custom(format!("Could not parse symbol: {:?}", error)))
    }
}

#[derive(Debug, StructOpt)]
pub enum OrderType {
    #[structopt(name = "buy")]
    Buy,
    #[structopt(name = "sell")]
    Sell,
}

pub fn run(
    trading_api_url: TradingApiUrl,
    symbol: Symbol,
    order_type: OrderType,
    amount: f64,
) -> Result<String, TradingServiceError> {
    let offer_request_body = BuyOfferRequestBody::new(amount);

    match order_type {
        OrderType::Sell => panic!("Only buy orders are currently supported"),
        OrderType::Buy => {
            let client = create_client(&trading_api_url);
            let offer = client.request_offer(&symbol, &offer_request_body)?;

            return Ok(format!(
                "#### Trade id: {} ####\n\
                 The offered exchange rate is {} {}\n\
                 Sell {} for {}\n\
                 To accept the offer, run:\n\
                 trading_client order --symbol=ETH-BTC --uid={} --refund-address=<your BTC address> --success-address=<your ETH address>",
                offer.uid, offer.rate, symbol, offer.btc_amount, offer.eth_amount, offer.uid,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_offer_with_supported_currency() {
        let trading_api_url = TradingApiUrl("stub".to_string());
        let symbol = Symbol::from_str("ETH-BTC").unwrap();
        let response = run(trading_api_url, symbol, OrderType::Buy, 12.0).unwrap();

        assert_eq!(
            response,
            "#### Trade id: a83aac12-0c78-417e-88e4-1a2948c6d538 ####\n\
             The offered exchange rate is 0.07 ETH-BTC\n\
             Sell 7 BTC for 100 ETH\n\
             To accept the offer, run:\n\
             trading_client order --symbol=ETH-BTC --uid=a83aac12-0c78-417e-88e4-1a2948c6d538 --refund-address=<your BTC address> --success-address=<your ETH address>"
        );
    }
}
