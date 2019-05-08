use crate::{
    offset_parameter::{apply_offsets, Error, OffsetParameter},
    rfc003::{secret_hash::SecretHash, timestamp::Timestamp},
};
use std::ops::Range;
use web3::types::{Address, U256};

pub const SECRET_HASH_RANGE: Range<usize> = 53..85;
pub const EXPIRY_RANGE: Range<usize> = 102..106;
pub const REDEEM_IDENTITY_RANGE: Range<usize> = 157..177;
pub const REFUND_IDENTITY_RANGE: Range<usize> = 224..244;
pub const TOKEN_QUANTITY_RANGE: Range<usize> = 261..293;
pub const TOKEN_CONTRACT_RANGE: Range<usize> = 307..327;

const CONTRACT_TEMPLATE: & str = "61014461000f6000396101446000f3361561005457602036141561006057602060006000376020602160206000600060026048f17f000000000000000000000000000000000000000000000000000000000000000060215114166100665760006000f35b426300000000106100a9575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a17300000000000000000000000000000000000000006020526100ec565b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a17300000000000000000000000000000000000000006020526100ec565b63a9059cbb6000527f0000000000000000000000000000000000000000000000000000000000000064604052602060606044601c6000730000000000000000000000000000000000000000620186a05a03f150602051ff";

#[derive(Debug, Clone)]
pub struct Erc20Htlc(Vec<u8>);

impl From<Erc20Htlc> for Vec<u8> {
    fn from(htlc: Erc20Htlc) -> Self {
        htlc.0
    }
}

impl Erc20Htlc {
    pub fn new(
        expiry: Timestamp,
        refund_identity: Address,
        redeem_identity: Address,
        secret_hash: SecretHash,
        token_contract_address: Address,
        token_quantity: U256,
    ) -> Result<Erc20Htlc, Error> {
        let offsets = vec![
            OffsetParameter::new(expiry, EXPIRY_RANGE)?,
            OffsetParameter::new(refund_identity, REFUND_IDENTITY_RANGE)?,
            OffsetParameter::new(redeem_identity, REDEEM_IDENTITY_RANGE)?,
            OffsetParameter::new(secret_hash, SECRET_HASH_RANGE)?,
            OffsetParameter::new(token_contract_address, TOKEN_CONTRACT_RANGE)?,
            OffsetParameter::new(token_quantity, TOKEN_QUANTITY_RANGE)?,
        ];
        let data = apply_offsets(CONTRACT_TEMPLATE, offsets)?;

        Ok(Erc20Htlc(data))
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
    fn compiled_contract_is_same_length_as_template() -> Result<(), Error> {
        let htlc = Erc20Htlc::new(
            Timestamp::from(3000000),
            Address::default(),
            Address::default(),
            SecretHash::from(SECRET_HASH),
            Address::default(),
            U256::from(100),
        )?;

        assert_eq!(
            htlc.0.len(),                // This is a Vec<u8>
            CONTRACT_TEMPLATE.len() / 2, // This is hex
            "HTLC is the same length as template"
        );

        Ok(())
    }

    #[test]
    fn given_input_data_when_compiled_should_contain_given_data() {
        let htlc = Erc20Htlc::new(
            Timestamp::from(2000000000),
            Address::default(),
            Address::default(),
            SecretHash::from(SECRET_HASH),
            Address::default(),
            U256::from(100),
        )
        .unwrap();

        let compiled_code = htlc.0;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
