use crate::calculate_offsets::{DataName, Offset};
use blockchain_contracts::rfc003::{secret_hash::SecretHash, timestamp::Timestamp};
use ethereum_support::{Address, U256};
use regex::bytes::Regex;

#[derive(Debug, Clone)]
pub struct Erc20Htlc {
    refund_timestamp: Timestamp,
    refund_address: Address,
    redeem_address: Address,
    secret_hash: SecretHash,
    token_contract_address: Address,
    amount: U256,
}

impl Erc20Htlc {
    const CONTRACT_CODE_TEMPLATE: &'static str =
        include_str!("./templates/out/erc20_contract.asm.hex");
    const SECRET_HASH_REGEX: &'static str =
        r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";
    const EXPIRY_REGEX: &'static str = r"\x20\x00\x00\x02";
    const REDEEM_ADDRESS_REGEX: &'static str =
        r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
    const REFUND_ADDRESS_REGEX: &'static str =
        r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
    const AMOUNT_REGEX: &'static str =
        r"\x50\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x05";
    const TOKEN_CONTRACT_ADDRESS_REGEX: &'static str =
        r"\x60\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x06";

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

        deploy_header + Self::CONTRACT_CODE_TEMPLATE
    }

    fn offset(data_name: DataName, regex: &str) -> Offset {
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

    pub fn all_offsets() -> Vec<Offset> {
        let refund_timestamp = Self::offset(DataName::Expiry, Self::EXPIRY_REGEX);
        let redeem_address = Self::offset(DataName::RedeemIdentity, Self::REDEEM_ADDRESS_REGEX);
        let refund_address = Self::offset(DataName::RefundIdentity, Self::REFUND_ADDRESS_REGEX);
        let secret_hash = Self::offset(DataName::SecretHash, Self::SECRET_HASH_REGEX);
        let amount = Self::offset(DataName::TokenQuantity, Self::AMOUNT_REGEX);
        let token_contract_address =
            Self::offset(DataName::TokenContract, Self::TOKEN_CONTRACT_ADDRESS_REGEX);

        vec![
            secret_hash,
            refund_timestamp,
            redeem_address,
            refund_address,
            amount,
            token_contract_address,
        ]
    }
}
