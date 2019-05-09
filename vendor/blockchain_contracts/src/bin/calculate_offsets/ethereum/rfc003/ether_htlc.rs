use crate::calculate_offsets::{DataName, Offset};
use blockchain_contracts::rfc003::{secret_hash::SecretHash, timestamp::Timestamp};
use ethereum_support::Address;
use regex::bytes::Regex;

#[derive(Debug)]
pub struct EtherHtlc {
    refund_timestamp: Timestamp,
    refund_address: Address,
    redeem_address: Address,
    secret_hash: SecretHash,
}

impl EtherHtlc {
    pub const CONTRACT_CODE_TEMPLATE: &'static str =
        include_str!("./templates/out/ether_contract.asm.hex");

    const EXPIRY_REGEX: &'static str = r"\x20\x00\x00\x02";
    const REDEEM_ADDRESS_REGEX: &'static str =
        r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
    const REFUND_ADDRESS_REGEX: &'static str =
        r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
    const SECRET_HASH_REGEX: &'static str =
        r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";

    const DEPLOY_HEADER_TEMPLATE: &'static str =
        include_str!("./templates/out/deploy_header.asm.hex");
    const CONTRACT_START_POSITION_PLACEHOLDER: &'static str = "1001";
    const CONTRACT_LENGTH_PLACEHOLDER: &'static str = "2002";

    pub fn compile_template_to_hex() -> String {
        let code_length = Self::CONTRACT_CODE_TEMPLATE.len() / 2; // In hex, each byte is two chars

        let code_length_as_hex = format!("{:0>4x}", code_length);

        let header_length = Self::DEPLOY_HEADER_TEMPLATE.len() / 2;
        let header_length_as_hex = format!("{:0>4x}", header_length);

        let deploy_header = Self::DEPLOY_HEADER_TEMPLATE
            .to_string()
            .replace(
                Self::CONTRACT_START_POSITION_PLACEHOLDER,
                &header_length_as_hex,
            )
            .replace(Self::CONTRACT_LENGTH_PLACEHOLDER, &code_length_as_hex);

        deploy_header + &Self::CONTRACT_CODE_TEMPLATE.to_string()
    }

    fn get_offset(data_name: DataName, regex: &str) -> Offset {
        let contract = hex::decode(Self::compile_template_to_hex())
            .expect("contract is expected to be hex encoded");

        let re_match = Regex::new(regex)
            .expect("Could not create regex")
            .find(&contract)
            .expect("Could not find regex in hex code");
        Offset::new(
            data_name,
            re_match.start(),
            re_match.end(),
            re_match.end() - re_match.start(),
        )
    }

    pub fn get_all_offsets() -> Vec<Offset> {
        let refund_timestamp = Self::get_offset(DataName::Expiry, Self::EXPIRY_REGEX);
        let redeem_address = Self::get_offset(DataName::RedeemIdentity, Self::REDEEM_ADDRESS_REGEX);
        let refund_address = Self::get_offset(DataName::RefundIdentity, Self::REFUND_ADDRESS_REGEX);
        let secret_hash = Self::get_offset(DataName::SecretHash, Self::SECRET_HASH_REGEX);

        vec![
            secret_hash,
            refund_timestamp,
            redeem_address,
            refund_address,
        ]
    }
}
