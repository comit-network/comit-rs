use crate::offset_parameter::{apply_offsets, OffsetParameter};
use binary_macros::{base16, base16_impl};
use hex_literal::hex;
use std::ops::Range;
use web3::types::{Address, Bytes, U256};

pub const SECRET_HASH_RANGE: Range<usize> = 53..85;
pub const EXPIRY_RANGE: Range<usize> = 102..106;
pub const REDEEM_IDENTITY_RANGE: Range<usize> = 157..177;
pub const REFUND_IDENTITY_RANGE: Range<usize> = 224..244;
pub const TOKEN_QUANTITY_RANGE: Range<usize> = 261..293;
pub const TOKEN_CONTRACT_RANGE: Range<usize> = 307..327;

const CONTRACT_TEMPLATE: [u8;339] = hex!("61014461000f6000396101446000f3361561005457602036141561006057602060006000376020602160206000600060026048f17f000000000000000000000000000000000000000000000000000000000000000060215114166100665760006000f35b426300000000106100a9575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a17300000000000000000000000000000000000000006020526100ec565b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a17300000000000000000000000000000000000000006020526100ec565b63a9059cbb6000527f0000000000000000000000000000000000000000000000000000000000000064604052602060606044601c6000730000000000000000000000000000000000000000620186a05a03f150602051ff");

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
        let offsets = vec![
            OffsetParameter::new(expiry, EXPIRY_RANGE).expect("always 4 bytes"),
            OffsetParameter::new(refund_identity, REFUND_IDENTITY_RANGE).expect("always 20 bytes"),
            OffsetParameter::new(redeem_identity, REDEEM_IDENTITY_RANGE).expect("always 20 bytes"),
            OffsetParameter::new(&secret_hash[..], SECRET_HASH_RANGE).expect("always 32 bytes"),
            OffsetParameter::new(token_contract_address, TOKEN_CONTRACT_RANGE)
                .expect("always 20 bytes"),
            OffsetParameter::new(token_quantity, TOKEN_QUANTITY_RANGE).expect("always 32 bytes"),
        ];
        let data = apply_offsets(&CONTRACT_TEMPLATE[..], offsets);

        Erc20Htlc(data)
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
        let transfer_fn_abi = base16!("A9059CBB");
        let to_address = <[u8; 20]>::from(to_address);
        let amount = <[u8; 32]>::from(token_quantity);

        let mut data = [0u8; 4 + 32 + 32];
        data[..4].copy_from_slice(transfer_fn_abi);
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
