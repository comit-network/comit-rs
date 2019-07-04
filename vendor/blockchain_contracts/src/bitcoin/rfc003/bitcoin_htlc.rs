use crate::{fit_into_placeholder_slice::FitIntoPlaceholderSlice, SecretHash, Timestamp};
use bitcoin_support::Hash160;
use hex_literal::hex;

// contract template RFC: https://github.com/comit-network/RFCs/blob/master/RFC-005-SWAP-Basic-Bitcoin.md#contract
pub const CONTRACT_TEMPLATE: [u8;97] = hex!("6382012088a82010000000000000000000000000000000000000000000000000000000000000018876a9143000000000000000000000000000000000000003670420000002b17576a91440000000000000000000000000000000000000046888ac");

#[derive(Debug, Clone)]
pub struct BitcoinHtlc(Vec<u8>);

impl BitcoinHtlc {
    pub fn new(
        expiry: u32,
        refund_identity: Hash160,
        redeem_identity: Hash160,
        secret_hash: [u8; 32],
    ) -> Self {
        let mut contract = CONTRACT_TEMPLATE.to_vec();
        SecretHash(secret_hash).fit_into_placeholder_slice(&mut contract[7..39]);
        redeem_identity.fit_into_placeholder_slice(&mut contract[43..63]);
        Timestamp(expiry).fit_into_placeholder_slice(&mut contract[65..69]);
        refund_identity.fit_into_placeholder_slice(&mut contract[74..94]);

        BitcoinHtlc(contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::bytes::Regex;

    const SECRET_HASH: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        0, 1,
    ];

    const SECRET_HASH_REGEX: &'static str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01";

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = BitcoinHtlc::new(3000000, Hash160::default(), Hash160::default(), SECRET_HASH);

        assert_eq!(
            htlc.0.len(),
            CONTRACT_TEMPLATE.len(),
            "HTLC is the same length as template"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_contain_given_data() {
        let htlc = BitcoinHtlc::new(
            2000000000,
            Hash160::default(),
            Hash160::default(),
            SECRET_HASH,
        );

        let compiled_code = htlc.0;

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&compiled_code)
            .expect("Could not find secret hash in hex code");
    }
}
