use offer::Symbol;
use trading_service_api_client::ApiClient;
use trading_service_api_client::BuyOrderRequestBody;
use trading_service_api_client::TradingApiUrl;
use trading_service_api_client::create_client;
use uuid::Uuid;

pub fn run(
    trading_api_url: TradingApiUrl,
    symbol: Symbol,
    uid: Uuid,
    success_address: String,
    refund_address: String,
) -> Result<String, String> {
    let order_request_body = BuyOrderRequestBody::new(success_address, refund_address);

    let client = create_client(&trading_api_url);
    let res = client.request_order(&symbol, uid, &order_request_body);

    let request_to_fund = match res {
        Ok(request_to_fund) => request_to_fund,
        Err(e) => return Err(format!("{:?}", e)),
    };

    Ok(format!(
        "Trade id: {}\n\
         You have accepted the order!\n\
         Please send {} {} to the following address to get your {}:\n\
         {}",
        uid,
        request_to_fund.sell_amount,
        symbol.get_base_currency(),
        symbol.get_traded_currency(),
        request_to_fund.address_to_fund
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn accept_order_happy_path() {
        let trading_api_url = TradingApiUrl("stub".to_string());
        let symbol = Symbol::from_str("ETH-BTC").unwrap();
        let uid = Uuid::from_str("27b36adf-eda3-4684-a21c-a08a84f36fb1").unwrap();

        let response = run(
            trading_api_url,
            symbol,
            uid,
            "0x00a329c0648769a73afac7f9381e08fb43dbea72".to_string(),
            "bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg".to_string(),
        ).unwrap();

        assert_eq!(
            response,
            "Trade id: 27b36adf-eda3-4684-a21c-a08a84f36fb1
You have accepted the order!
Please send 1001 BTC to the following address to get your ETH:
bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap"
        );
    }
}
