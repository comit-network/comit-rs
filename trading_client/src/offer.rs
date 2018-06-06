use std::fmt;
use std::str::FromStr;
use trading_service_api_client::create_client;
use trading_service_api_client::{ApiClient, BuyOfferRequestBody};
use types::TradingApiUrl;

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

#[derive(Debug, Deserialize, Serialize)]
pub struct Symbol(String);

impl fmt::Display for Symbol {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

pub enum OrderType {
    BUY,
    SELL,
}

impl Symbol {
    pub fn new(sell: Currency, buy: Currency) -> (Symbol, OrderType) {
        //TODO: is there a smarter way to do that?

        let usd = Currency::from_str("USD").unwrap();
        let btc = Currency::from_str("BTC").unwrap();
        let eth = Currency::from_str("ETH").unwrap();

        let (symbol, order_type) = if sell == usd {
            let symbol = Symbol(format!("{}-{}", buy, sell));
            let order_type = OrderType::BUY;
            (symbol, order_type)
        } else if buy == usd {
            let symbol = Symbol(format!("{}-{}", sell, buy));
            let order_type = OrderType::SELL;
            (symbol, order_type)
        } else if sell == btc {
            let symbol = Symbol(format!("{}-{}", buy, sell));
            let order_type = OrderType::BUY;
            (symbol, order_type)
        } else if buy == btc {
            let symbol = Symbol(format!("{}-{}", sell, buy));
            let order_type = OrderType::SELL;
            (symbol, order_type)
        } else if sell == eth {
            let symbol = Symbol(format!("{}-{}", buy, sell));
            let order_type = OrderType::BUY;
            (symbol, order_type)
        } else if buy == eth {
            let symbol = Symbol(format!("{}-{}", sell, buy));
            let order_type = OrderType::SELL;
            (symbol, order_type)
        } else {
            panic!("This combination of currencies is not supported.")
        };
        return (symbol, order_type);
    }
}

pub fn run(
    trading_api_url: TradingApiUrl,
    sell_curr: Currency,
    buy_curr: Currency,
    buy_amount: u32,
) -> Result<String, String> {
    let offer_request_body = BuyOfferRequestBody::new(buy_amount);

    let (symbol, order_type) = Symbol::new(sell_curr.clone(), buy_curr.clone());

    match order_type {
        OrderType::SELL => panic!("Only buy orders are currently supported"),
        OrderType::BUY => {
            let client = create_client(&trading_api_url);
            let res = client.request_offer(symbol, &offer_request_body);

            let offer = match res {
                Ok(offer) => offer,
                Err(e) => return Err(format!("Error: {}; offer aborted", e)),
            };

            return Ok(format!(
                "Offer details:\n\
                 To buy {} {} against {}, the offered exchange rate is {} {}.\n\
                 Offer id is: {}\n\
                 To accept the offer, run:\n\
                 trading_client accept --uid={}",
                buy_amount, buy_curr, sell_curr, offer.rate, offer.symbol, offer.uid, offer.uid,
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

        let eth_buy = Currency::from_str("ETH").unwrap();
        let btc_sell = Currency::from_str("BTC").unwrap();

        let response = run(trading_api_url, btc_sell, eth_buy, 12).unwrap();

        assert_eq!(
            response,
            "Offer details:
To buy 12 ETH against BTC, the offered exchange rate is 0.6876231 ETH-BTC.
Offer id is: a83aac12-0c78-417e-88e4-1a2948c6d538
To accept the offer, run:
trading_client accept --uid=a83aac12-0c78-417e-88e4-1a2948c6d538"
        )
    }
}
