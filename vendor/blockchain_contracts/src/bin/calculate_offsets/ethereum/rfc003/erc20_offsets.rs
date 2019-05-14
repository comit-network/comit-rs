use crate::calculate_offsets::{DataName, Offset};
use regex::bytes::Regex;

const CONTRACT_CODE_TEMPLATE: &str = include_str!("./templates/out/erc20_contract.asm.hex");
const SECRET_HASH_REGEX: &str =
    r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";
const EXPIRY_REGEX: &str = r"\x20\x00\x00\x02";
const REDEEM_ADDRESS_REGEX: &str =
    r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
const REFUND_ADDRESS_REGEX: &str =
    r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
const AMOUNT_REGEX: &str =
    r"\x50\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x05";
const TOKEN_CONTRACT_ADDRESS_REGEX: &str =
    r"\x60\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x06";

const DEPLOY_HEADER_TEMPLATE: &str = include_str!("./templates/out/deploy_header.asm.hex");
const CONTRACT_START_POSITION_PLACEHOLDER: &str = "1001";
const CONTRACT_LENGTH_PLACEHOLDER: &str = "2002";

pub fn contract_template() -> String {
    let code_length = CONTRACT_CODE_TEMPLATE.len() / 2; // In hex, each byte is two chars

    let code_length_as_hex = format!("{:0>4x}", code_length);

    let header_length = DEPLOY_HEADER_TEMPLATE.len() / 2;
    let header_length_as_hex = format!("{:0>4x}", header_length);

    let deploy_header = DEPLOY_HEADER_TEMPLATE
        .to_string()
        .replace(CONTRACT_START_POSITION_PLACEHOLDER, &header_length_as_hex)
        .replace(CONTRACT_LENGTH_PLACEHOLDER, &code_length_as_hex);

    deploy_header + CONTRACT_CODE_TEMPLATE
}

fn offset(data_name: DataName, regex: &str) -> Offset {
    let contract =
        hex::decode(contract_template()).expect("contract is expected to be hex encoded");

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

pub fn all_offsets() -> Vec<Offset> {
    let refund_timestamp = offset(DataName::Expiry, EXPIRY_REGEX);
    let redeem_address = offset(DataName::RedeemIdentity, REDEEM_ADDRESS_REGEX);
    let refund_address = offset(DataName::RefundIdentity, REFUND_ADDRESS_REGEX);
    let secret_hash = offset(DataName::SecretHash, SECRET_HASH_REGEX);
    let amount = offset(DataName::TokenQuantity, AMOUNT_REGEX);
    let token_contract_address = offset(DataName::TokenContract, TOKEN_CONTRACT_ADDRESS_REGEX);

    vec![
        secret_hash,
        refund_timestamp,
        redeem_address,
        refund_address,
        amount,
        token_contract_address,
    ]
}
