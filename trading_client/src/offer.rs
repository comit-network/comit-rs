use serde::Deserialize;
use serde::Deserializer;
use serde::de;
use serde::de::Visitor;
use std::fmt;
use std::str::FromStr;
use trading_service_api_client::ApiClient;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::TradingApiUrl;
use trading_service_api_client::create_client;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Currency(String);

impl fmt::Display for Currency {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

impl FromStr for Currency {
    //TODO: find out the proper way
    type Err = u32;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Currency(s.to_string()))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Symbol {
    traded: Currency,
    base: Currency,
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
    //TODO: find out the proper way
    type Err = u32;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let currencies: Vec<&str> = s.split("-").collect();

        let traded = currencies[0].parse::<Currency>()?;
        let base = currencies[1].parse::<Currency>()?;

        Ok(Symbol { traded, base })
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Self::from_str(s.as_str()).unwrap()
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
        let currencies: Vec<&str> = value.split("-").collect();

        let traded = currencies[0].parse::<Currency>();
        let traded = match traded {
            //TODO: Talk to Thomas to do it properly
            Err(_) => panic!("Could not convert received traded currency"),
            Ok(traded) => traded,
        };
        let base = currencies[1].parse::<Currency>();
        let base = match base {
            //TODO: Talk to Thomas to do it properly
            Err(_) => panic!("Could not convert received base currency"),
            Ok(base) => base,
        };

        Ok(Symbol { traded, base })
    }
}

pub enum OrderType {
    BUY,
    SELL,
}

impl OrderType {
    pub fn new(buy: bool, sell: bool) -> OrderType {
        let order_buy = if buy && sell {
            // TODO: learn how to implement exclusive parameters
            panic!("An order is either `buy` or `sell`, it cannot be both");
        } else if buy {
            OrderType::BUY
        } else if sell {
            OrderType::SELL
        } else {
            OrderType::BUY // defaults to buy
        };
        order_buy
    }
}

pub fn run(
    trading_api_url: TradingApiUrl,
    symbol: Symbol,
    order_type: OrderType,
    amount: u32,
) -> Result<String, String> {
    let offer_request_body = BuyOfferRequestBody::new(amount);

    match order_type {
        OrderType::SELL => panic!("Only buy orders are currently supported"),
        OrderType::BUY => {
            let client = create_client(&trading_api_url);
            let res = client.request_offer(&symbol, &offer_request_body);

            let offer = match res {
                Ok(offer) => offer,
                Err(e) => return Err(format!("Error: {}; offer aborted", e)),
            };

            return Ok(format!(
                "Trade id: {}\n\
                 To buy {} {} against {}, the offered exchange rate is {} {}.\n\
                 To accept the offer, run:\n\
                 trading_client accept --uid={}",
                offer.uid,
                amount,
                symbol.get_traded_currency(),
                symbol.get_base_currency(),
                offer.rate,
                offer.symbol,
                offer.uid,
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

        let response = run(trading_api_url, symbol, OrderType::BUY, 12).unwrap();

        assert_eq!(
            response,
            "Trade id: a83aac12-0c78-417e-88e4-1a2948c6d538\n\
             To buy 12 ETH against BTC, the offered exchange rate is 0.6876231 ETH-BTC.\n\
             To accept the offer, run:\n\
             trading_client accept --uid=a83aac12-0c78-417e-88e4-1a2948c6d538"
        )
    }
}
