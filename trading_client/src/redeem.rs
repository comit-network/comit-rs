use std::ops::Add;
use trading_service_api_client::ApiClient;
use trading_service_api_client::create_client;
use types::TradingApiUrl;
use uuid::Uuid;

pub enum OutputType {
    URL,
    CONSOLE,
}

pub fn run(
    trading_api_url: TradingApiUrl,
    uid: Uuid,
    output_type: OutputType,
) -> Result<String, String> {
    let client = create_client(&trading_api_url);

    let res = client.request_redeem_details(uid);

    let redeem_details = match res {
        Ok(redeem_details) => redeem_details,
        Err(e) => return Err(format!("Error: {}; Redeem aborted", e)),
    };

    let address = redeem_details.address.to_string();

    match output_type {
        OutputType::URL => {
            // See https://eips.ethereum.org/EIPS/eip-681
            let mut url = String::new()
                .add("ethereum:")
                .add("0x") // We receive a non-prefixed address
                .add(&address)
                //.push_str("@").push_str(chain_id) // TODO: Do we want it?
                .add("?value=0")
                .add("&gas=").add(&redeem_details.gas.to_string()); //TODO double check whether we should be using gas, gasLimit or gasPrice
            return Ok(format!(
                "To redeem your ETH related to trade id: {}\n\
                 Please proceed with a payment of 0 ETH using the following link:\n{}",
                uid, url
            ));
        }
        OutputType::CONSOLE => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redeem_with_valid_uid() {
        let trading_api_url = TradingApiUrl("stub".to_string());

        let uid = Uuid::new_v4();

        let redeem_details = run(trading_api_url, uid, OutputType::URL);
    }
}
