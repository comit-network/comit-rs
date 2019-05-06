use crate::{
    rfc003::timestamp::{Timestamp, ToVecError},
    OffsetParameter,
};
use std::ops::Range;
use web3::types::Address;

pub const SECRET_HASH_RANGE: Range<usize> = 51..83;
pub const SECRET_HASH_LENGTH: usize = 32;

pub const EXPIRY_RANGE: Range<usize> = 99..103;
pub const EXPIRY_LENGTH: usize = 4;

pub const REDEEM_IDENTITY_RANGE: Range<usize> = 153..173;
pub const REDEEM_IDENTITY_LENGTH: usize = 20;

pub const REFUND_IDENTITY_RANGE: Range<usize> = 214..234;
pub const REFUND_IDENTITY_LENGTH: usize = 20;

// TODO: This should match what is in the RFC
const CONTRACT_TEMPLATE: & str      = "6100dc61000f6000396100dc6000f336156051576020361415605c57602060006000376020602160206000600060026048f17f1000000000000000000000000000000000000000000000000000000000000001602151141660625760006000f35b42632000000210609f575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a1733000000000000000000000000000000000000003ff5b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a1734000000000000000000000000000000000000004ff";

#[derive(Debug)]
pub struct EtherHtlc(Vec<u8>);

impl From<EtherHtlc> for Vec<u8> {
    fn from(htlc: EtherHtlc) -> Self {
        htlc.0
    }
}

#[derive(Debug)]
pub enum Error {
    SecretHashLength,
    ExpiryToVec(ToVecError),
}

impl EtherHtlc {
    pub fn new(
        expiry: Timestamp,
        refund_identity: Address,
        redeem_identity: Address,
        secret_hash: [u8; SECRET_HASH_LENGTH],
    ) -> Result<EtherHtlc, Error> {
        let expiry = expiry.to_vec(EXPIRY_LENGTH).map_err(Error::ExpiryToVec)?;

        let offsets = vec![
            OffsetParameter {
                value: expiry,
                range: EXPIRY_RANGE,
            },
            OffsetParameter {
                value: Into::<[u8; REFUND_IDENTITY_LENGTH]>::into(refund_identity).to_vec(),
                range: REFUND_IDENTITY_RANGE,
            },
            OffsetParameter {
                value: Into::<[u8; REDEEM_IDENTITY_LENGTH]>::into(redeem_identity).to_vec(),
                range: REDEEM_IDENTITY_RANGE,
            },
            OffsetParameter {
                value: secret_hash.to_vec(),
                range: SECRET_HASH_RANGE,
            },
        ];

        let mut htlc = hex::decode(CONTRACT_TEMPLATE)
            .expect("Ether rfc003 template file should be encoded in hex");

        for offset in offsets {
            htlc.splice(offset.range, offset.value);
        }

        Ok(EtherHtlc(htlc))
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
        let htlc = EtherHtlc::new(
            Timestamp::from(3000000),
            Address::default(),
            Address::default(),
            SECRET_HASH,
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
        let htlc = EtherHtlc::new(
            Timestamp::from(2000000000),
            Address::default(),
            Address::default(),
            SECRET_HASH,
        )
        .unwrap();

        let compiled_code = htlc.0;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
