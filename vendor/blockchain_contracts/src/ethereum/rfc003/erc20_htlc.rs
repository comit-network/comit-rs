use crate::ethereum::{FitIntoPlaceholderSlice, SecretHash, Timestamp, TokenQuantity};
use hex_literal::hex;
use web3::types::{Address, Bytes, U256};

// contract template RFC: https://github.com/comit-network/RFCs/blob/master/RFC-009-SWAP-Basic-ERC20.md#contract
pub const CONTRACT_TEMPLATE: [u8;339] = hex!("61014461000f6000396101446000f3361561005457602036141561006057602060006000376020602160206000600060026048f17f100000000000000000000000000000000000000000000000000000000000000160215114166100665760006000f35b426320000002106100a9575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a17330000000000000000000000000000000000000036020526100ec565b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a17340000000000000000000000000000000000000046020526100ec565b63a9059cbb6000527f5000000000000000000000000000000000000000000000000000000000000005604052602060606044601c6000736000000000000000000000000000000000000006620186a05a03f150602051ff");

#[derive(Debug, Clone)]
pub struct Erc20Htlc(Vec<u8>);

impl From<Erc20Htlc> for Vec<u8> {
    fn from(htlc: Erc20Htlc) -> Self {
        htlc.0
    }
}

impl Erc20Htlc {
    pub fn new(
        expiry: u32,
        refund_identity: Address,
        redeem_identity: Address,
        secret_hash: [u8; 32],
        token_contract_address: Address,
        token_quantity: U256,
    ) -> Self {
        let mut contract = CONTRACT_TEMPLATE.to_vec();
        Timestamp(expiry).fit_into_placeholder_slice(&mut contract[102..106]);
        refund_identity.fit_into_placeholder_slice(&mut contract[224..244]);
        redeem_identity.fit_into_placeholder_slice(&mut contract[157..177]);
        SecretHash(secret_hash).fit_into_placeholder_slice(&mut contract[53..85]);
        token_contract_address.fit_into_placeholder_slice(&mut contract[307..327]);
        TokenQuantity(token_quantity).fit_into_placeholder_slice(&mut contract[261..293]);

        Erc20Htlc(contract)
    }

    pub fn deployment_gas_limit(&self) -> U256 {
        U256::from(167_800)
    }

    pub fn tx_gas_limit() -> U256 {
        U256::from(100_000)
    }

    pub fn fund_tx_gas_limit() -> U256 {
        U256::from(100_000)
    }

    /// Constructs the payload to transfer `Erc20` tokens to a `to_address`
    pub fn transfer_erc20_tx_payload(token_quantity: U256, to_address: Address) -> Bytes {
        let transfer_fn_abi = hex!("A9059CBB");
        let to_address = <[u8; 20]>::from(to_address);
        let amount = <[u8; 32]>::from(token_quantity);

        let mut data = [0u8; 4 + 32 + 32];
        data[..4].copy_from_slice(&transfer_fn_abi);
        data[16..36].copy_from_slice(&to_address);
        data[36..68].copy_from_slice(&amount);

        Bytes::from(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::bytes::Regex;
    use web3::types::Address;

    const SECRET_HASH: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        0, 1,
    ];

    const SECRET_HASH_REGEX: &'static str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01";

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = Erc20Htlc::new(
            3000000,
            Address::default(),
            Address::default(),
            SECRET_HASH,
            Address::default(),
            U256::from(100),
        );

        assert_eq!(
            htlc.0.len(),
            CONTRACT_TEMPLATE.len(),
            "HTLC is the same length as template"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_contain_given_data() {
        let htlc = Erc20Htlc::new(
            2000000000,
            Address::default(),
            Address::default(),
            SECRET_HASH,
            Address::default(),
            U256::from(100),
        );

        let compiled_code = htlc.0;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
