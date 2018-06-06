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

    let address = format!("{:x}", redeem_details.address);

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
                "Trade id: {}\n\
                 To redeem your ETH, proceed with a payment of 0 ETH using the following link:\n{}",
                uid, url
            ));
        }
        OutputType::CONSOLE => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn redeem_with_valid_uid() {
        let trading_api_url = TradingApiUrl("stub".to_string());

        let uid = Uuid::from_str("27b36adf-eda3-4684-a21c-a08a84f36fb1").unwrap();

        let redeem_details = run(trading_api_url, uid, OutputType::URL).unwrap();

        assert_eq!(
            redeem_details,
            "Trade id: 27b36adf-eda3-4684-a21c-a08a84f36fb1\n\
             To redeem your ETH, proceed with a payment of 0 ETH using the following link:\n\
             ethereum:0x00a329c0648769a73afac7f9381e08fb43dbea72?value=0&gas=20000"
        )
    }
}
