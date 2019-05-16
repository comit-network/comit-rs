use crate::calculate_offsets::{ethereum::rfc003::compile::compile, DataName, Offset};
use regex::bytes::Regex;

const EXPIRY_REGEX: &str = r"\x20\x00\x00\x02";
const REDEEM_ADDRESS_REGEX: &str =
    r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
const REFUND_ADDRESS_REGEX: &str =
    r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
const SECRET_HASH_REGEX: &str =
    r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";

const CONTRACT_START_POSITION_PLACEHOLDER: &str = "1001";
const CONTRACT_LENGTH_PLACEHOLDER: &str = "2002";

#[derive(Debug)]
pub struct EtherOffsets {
    pub contract_header: String,
    pub contract_body: String,
}

impl EtherOffsets {
    pub fn new() -> Self {
        EtherOffsets {
            contract_header: compile(
                "./src/bin/calculate_offsets/ethereum/rfc003/templates/deploy_header.asm",
            )
            .unwrap(),
            contract_body: compile(
                "./src/bin/calculate_offsets/ethereum/rfc003/templates/ether_contract.asm",
            )
            .unwrap(),
        }
    }

    pub fn contract_template(&self) -> String {
        let code_length = self.contract_body.len() / 2;

        let code_length_as_hex = format!("{:0>4x}", code_length);

        let header_length = self.contract_header.len() / 2;
        let header_length_as_hex = format!("{:0>4x}", header_length);

        let deploy_header = self
            .contract_header
            .to_string()
            .replace(CONTRACT_START_POSITION_PLACEHOLDER, &header_length_as_hex)
            .replace(CONTRACT_LENGTH_PLACEHOLDER, &code_length_as_hex);

        deploy_header + &self.contract_body.to_string()
    }

    fn get_offset(&self, data_name: DataName, regex: &str) -> Offset {
        let contract =
            hex::decode(self.contract_template()).expect("contract is expected to be hex encoded");

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

    pub fn get_all_offsets(&self) -> Vec<Offset> {
        let refund_timestamp = self.get_offset(DataName::Expiry, EXPIRY_REGEX);
        let redeem_address = self.get_offset(DataName::RedeemIdentity, REDEEM_ADDRESS_REGEX);
        let refund_address = self.get_offset(DataName::RefundIdentity, REFUND_ADDRESS_REGEX);
        let secret_hash = self.get_offset(DataName::SecretHash, SECRET_HASH_REGEX);

        vec![
            secret_hash,
            refund_timestamp,
            redeem_address,
            refund_address,
        ]
    }
}
