use crate::calculate_offsets::{DataName, Offset};
use regex::bytes::Regex;

pub const CONTRACT_CODE_TEMPLATE: &str = include_str!("./templates/out/ether_contract.asm.hex");

const EXPIRY_REGEX: &str = r"\x20\x00\x00\x02";
const REDEEM_ADDRESS_REGEX: &str =
    r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
const REFUND_ADDRESS_REGEX: &str =
    r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
const SECRET_HASH_REGEX: &str =
    r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";

const DEPLOY_HEADER_TEMPLATE: &str = include_str!("./templates/out/deploy_header.asm.hex");
const CONTRACT_START_POSITION_PLACEHOLDER: &str = "1001";
const CONTRACT_LENGTH_PLACEHOLDER: &str = "2002";

pub fn compile_template_to_hex() -> String {
    let code_length = CONTRACT_CODE_TEMPLATE.len() / 2; // In hex, each byte is two chars

    let code_length_as_hex = format!("{:0>4x}", code_length);

    let header_length = DEPLOY_HEADER_TEMPLATE.len() / 2;
    let header_length_as_hex = format!("{:0>4x}", header_length);

    let deploy_header = DEPLOY_HEADER_TEMPLATE
        .to_string()
        .replace(CONTRACT_START_POSITION_PLACEHOLDER, &header_length_as_hex)
        .replace(CONTRACT_LENGTH_PLACEHOLDER, &code_length_as_hex);

    deploy_header + &CONTRACT_CODE_TEMPLATE.to_string()
}

fn get_offset(data_name: DataName, regex: &str) -> Offset {
    let contract =
        hex::decode(compile_template_to_hex()).expect("contract is expected to be hex encoded");

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
    let refund_timestamp = get_offset(DataName::Expiry, EXPIRY_REGEX);
    let redeem_address = get_offset(DataName::RedeemIdentity, REDEEM_ADDRESS_REGEX);
    let refund_address = get_offset(DataName::RefundIdentity, REFUND_ADDRESS_REGEX);
    let secret_hash = get_offset(DataName::SecretHash, SECRET_HASH_REGEX);

    vec![
        secret_hash,
        refund_timestamp,
        redeem_address,
        refund_address,
    ]
}
