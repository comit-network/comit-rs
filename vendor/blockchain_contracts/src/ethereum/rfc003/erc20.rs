use crate::{
    rfc003::timestamp::{Timestamp, ToVecError},
    OffsetParameter,
};
use binary_macros::{base16, base16_impl};
use std::ops::Range;
use web3::types::{Address, Bytes, U256};

pub const SECRET_HASH_RANGE: Range<usize> = 53..85;
pub const SECRET_HASH_LENGTH: usize = 32;

pub const EXPIRY_RANGE: Range<usize> = 102..106;
pub const EXPIRY_LENGTH: usize = 4;

pub const REDEEM_IDENTITY_RANGE: Range<usize> = 157..177;
pub const REDEEM_IDENTITY_LENGTH: usize = 20;

pub const REFUND_IDENTITY_RANGE: Range<usize> = 224..244;
pub const REFUND_IDENTITY_LENGTH: usize = 20;

pub const TOKEN_QUANTITY_RANGE: Range<usize> = 261..293;
pub const TOKEN_QUANTITY_LENGTH: usize = 32;

pub const TOKEN_CONTRACT_RANGE: Range<usize> = 307..327;
pub const TOKEN_CONTRACT_LENGTH: usize = 20;

// TODO: This should match what is in the RFC
const CONTRACT_TEMPLATE: & str = "61014461000f6000396101446000f3361561005457602036141561006057602060006000376020602160206000600060026048f17f000000000000000000000000000000000000000000000000000000000000000060215114166100665760006000f35b426300000000106100a9575b60006000f35b7fb8cac300e37f03ad332e581dea21b2f0b84eaaadc184a295fef71e81f44a741360206000a17300000000000000000000000000000000000000006020526100ec565b7f5d26862916391bf49478b2f5103b0720a842b45ef145a268f2cd1fb2aed5517860006000a17300000000000000000000000000000000000000006020526100ec565b63a9059cbb6000527f0000000000000000000000000000000000000000000000000000000000000064604052602060606044601c6000730000000000000000000000000000000000000000620186a05a03f150602051ff";

#[derive(Debug, Clone)]
pub struct Erc20Htlc {
    data: Vec<u8>,
    token_quantity: U256,
}

impl From<Erc20Htlc> for Vec<u8> {
    fn from(htlc: Erc20Htlc) -> Self {
        htlc.data
    }
}

#[derive(Debug)]
pub enum Error {
    SecretHashLength,
    ExpiryToVec(ToVecError),
}

impl Erc20Htlc {
    pub fn new(
        expiry: Timestamp,
        refund_identity: Address,
        redeem_identity: Address,
        secret_hash: [u8; SECRET_HASH_LENGTH],
        token_contract_address: Address,
        token_quantity: U256,
    ) -> Result<Erc20Htlc, Error> {
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
            OffsetParameter {
                value: Into::<[u8; TOKEN_CONTRACT_LENGTH]>::into(token_contract_address).to_vec(),
                range: TOKEN_CONTRACT_RANGE,
            },
            OffsetParameter {
                value: Into::<[u8; TOKEN_QUANTITY_LENGTH]>::into(token_quantity).to_vec(),
                range: TOKEN_QUANTITY_RANGE,
            },
        ];
        let mut data = hex::decode(CONTRACT_TEMPLATE)
            .expect("Ether rfc003 template file should be encoded in hex");

        for offset in offsets {
            data.splice(offset.range, offset.value);
        }

        Ok(Erc20Htlc {
            data,
            token_quantity,
        })
    }

    /// Constructs the payload for funding an `Erc20` HTLC located at the given
    /// address.
    pub fn funding_tx_payload(&self, htlc_contract_address: Address) -> Bytes {
        let transfer_fn_abi = base16!("A9059CBB");
        let htlc_contract_address = <[u8; 20]>::from(htlc_contract_address);
        let amount = <[u8; 32]>::from(self.token_quantity);

        let mut data = [0u8; 4 + 32 + 32];
        data[..4].copy_from_slice(transfer_fn_abi);
        data[16..36].copy_from_slice(&htlc_contract_address);
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
    fn compiled_contract_is_same_length_as_template() -> Result<(), Error> {
        let htlc = Erc20Htlc::new(
            Timestamp::from(3000000),
            Address::default(),
            Address::default(),
            SECRET_HASH,
            Address::default(),
            U256::from(100),
        )?;

        assert_eq!(
            htlc.data.len(),             // This is a Vec<u8>
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
            SECRET_HASH,
            Address::default(),
            U256::from(100),
        )
        .unwrap();

        let compiled_code = htlc.data;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
