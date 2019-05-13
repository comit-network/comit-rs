use crate::offset_parameter::{apply_offsets, OffsetParameter};
use std::ops::Range;
use web3::types::{Address, U256};

pub const SECRET_HASH_RANGE: Range<usize> = 51..83;
pub const EXPIRY_RANGE: Range<usize> = 99..103;
pub const REDEEM_IDENTITY_RANGE: Range<usize> = 153..173;
pub const REFUND_IDENTITY_RANGE: Range<usize> = 214..234;

use hex_literal::hex;

// contract template RFC: https://github.com/comit-network/RFCs/blob/master/RFC-007-SWAP-Basic-Ether.md#contract
const CONTRACT_TEMPLATE: [u8;235] = hex!("6100dc61000f6000396100dc6000f336156051576020361415605c57602060006000376020602160206000600060026048f17f0000000000000000000000000000000000000000000000000000000000000000602151141660625760006000f35b42630000000010609f575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a1730000000000000000000000000000000000000000ff5b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a1730000000000000000000000000000000000000000ff");

#[derive(Debug)]
pub struct EtherHtlc(Vec<u8>);

impl From<EtherHtlc> for Vec<u8> {
    fn from(htlc: EtherHtlc) -> Self {
        htlc.0
    }
}

impl EtherHtlc {
    pub fn new(
        expiry: u32,
        refund_identity: Address,
        redeem_identity: Address,
        secret_hash: [u8; 32],
    ) -> Self {
        let offsets = vec![
            OffsetParameter::new(expiry, EXPIRY_RANGE).expect("always 4 bytes"),
            OffsetParameter::new(refund_identity, REFUND_IDENTITY_RANGE).expect("always 20 bytes"),
            OffsetParameter::new(redeem_identity, REDEEM_IDENTITY_RANGE).expect("always 20 bytes"),
            OffsetParameter::new(&secret_hash[..], SECRET_HASH_RANGE).expect("always 32 bytes"),
        ];

        let data = apply_offsets(&CONTRACT_TEMPLATE[..], offsets);

        EtherHtlc(data)
    }

    pub fn deployment_gas_limit(&self) -> U256 {
        U256::from(121_800)
    }

    pub fn tx_gas_limit() -> U256 {
        U256::from(100_000)
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
        let htlc = EtherHtlc::new(3000000, Address::default(), Address::default(), SECRET_HASH);

        assert_eq!(
            htlc.0.len(),
            CONTRACT_TEMPLATE.len(),
            "HTLC is the same length as template"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_contain_given_data() {
        let htlc = EtherHtlc::new(
            2000000000,
            Address::default(),
            Address::default(),
            SECRET_HASH,
        );

        let compiled_code = htlc.0;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
