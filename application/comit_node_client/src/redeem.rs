use api_client::{create_client, ApiClient, TradingApiUrl, TradingServiceError};
use common_types;
use ethereum_support;
use offer::Symbol;
use std::{fmt, ops::Add};
use uuid::Uuid;

pub enum RedeemOutput {
    URL,
    CONSOLE,
}

impl RedeemOutput {
    pub fn new(console: bool) -> RedeemOutput {
        if console {
            RedeemOutput::CONSOLE
        } else {
            RedeemOutput::URL
        }
    }
}

pub struct EthereumPaymentURL(String);

impl fmt::Display for EthereumPaymentURL {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

impl EthereumPaymentURL {
    pub fn new(
        address: &ethereum_support::Address,
        gas: u64,
        secret: common_types::secret::Secret,
    ) -> EthereumPaymentURL {
        let address = format!("{:x}", address);
        // See https://eips.ethereum.org/EIPS/eip-681
        EthereumPaymentURL(
            String::new()
            .add("ethereum:")
            .add("0x") // We receive a non-prefixed address
            .add(&address)
            //.push_str("@").push_str(chain_id) // TODO: must be implemented
            .add("?value=0")
            .add("&gas=").add(&gas.to_string())
            .add("&bytes32=").add(format!("{:x}", secret).as_str()),
        )
    }
}

pub fn run(
    trading_api_url: TradingApiUrl,
    symbol: Symbol,
    uid: Uuid,
    output_type: RedeemOutput,
) -> Result<String, TradingServiceError> {
    let client = create_client(&trading_api_url);

    let redeem_details = client.request_redeem_details(symbol, uid)?;

    match output_type {
        RedeemOutput::URL => {
            let mut url = EthereumPaymentURL::new(
                &redeem_details.address,
                redeem_details.gas,
                redeem_details.data,
            );
            return Ok(format!(
                "#### Trade id: {} ####\n\
                 In order to complete the trade, please sign and send the following transaction:\n{}",
                uid, url
            ));
        }
        RedeemOutput::CONSOLE => unimplemented!(),
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
        let symbol = Symbol::from_str("ETH-BTC").unwrap();

        let redeem_details = run(trading_api_url, symbol, uid, RedeemOutput::URL).unwrap();

        assert_eq!(
            redeem_details,
            "#### Trade id: 27b36adf-eda3-4684-a21c-a08a84f36fb1 ####\n\
             In order to complete the trade, please sign and send the following transaction:\n\
             ethereum:0x00a329c0648769a73afac7f9381e08fb43dbea72?value=0&gas=20000&bytes32=1234567890123456789012345678901212345678901234567890123456789012"
        )
    }
}
