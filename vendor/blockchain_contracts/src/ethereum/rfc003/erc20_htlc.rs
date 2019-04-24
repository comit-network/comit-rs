use crate::{
    ethereum::{rfc003::Htlc, ByteCode},
    rfc003::{secret_hash::SecretHash, timestamp::Timestamp},
    Offset,
};
use binary_macros::{base16, base16_impl};
use ethereum_support::{Address, Bytes, U256};
use regex::Regex;

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
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";
    const REFUND_TIMESTAMP_PLACEHOLDER: &'static str = "20000002";
    const REDEEM_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const AMOUNT_PLACEHOLDER: &'static str =
        "5000000000000000000000000000000000000000000000000000000000000005";
    const TOKEN_CONTRACT_ADDRESS_PLACEHOLDER: &'static str =
        "6000000000000000000000000000000000000006";

    const DEPLOY_HEADER_TEMPLATE: &'static str =
        include_str!("./templates/out/erc20_deploy_header.asm.hex");
    const CONTRACT_START_POSITION_PLACEHOLDER: &'static str = "1001";
    const CONTRACT_LENGTH_PLACEHOLDER: &'static str = "2002";

    pub fn new(
        refund_timestamp: Timestamp,
        refund_address: Address,
        redeem_address: Address,
        secret_hash: SecretHash,
        token_contract_address: Address,
        amount: U256,
    ) -> Self {
        Self {
            refund_timestamp,
            refund_address,
            redeem_address,
            secret_hash,
            token_contract_address,
            amount,
        }
    }

    /// Constructs the payload for funding an `Erc20` HTLC located at the given
    /// address.
    pub fn funding_tx_payload(&self, htlc_contract_address: Address) -> Bytes {
        let transfer_fn_abi = base16!("A9059CBB");
        let htlc_contract_address = <[u8; 20]>::from(htlc_contract_address);
        let amount = <[u8; 32]>::from(self.amount);

        let mut data = [0u8; 4 + 32 + 32];
        data[..4].copy_from_slice(transfer_fn_abi);
        data[16..36].copy_from_slice(&htlc_contract_address);
        data[36..68].copy_from_slice(&amount);

        Bytes::from(data.to_vec())
    }

    pub fn token_contract_address(&self) -> Address {
        self.token_contract_address
    }

    pub fn deployment_gas_limit(&self) -> U256 {
        let bytes: Bytes = self.compile_to_hex().into();
        let n_bytes = bytes.0.len();
        let gas_per_byte = 200;

        U256::from(100_000 + n_bytes * gas_per_byte)
    }

    pub fn tx_gas_limit() -> U256 {
        U256::from(100_000)
    }

    pub fn fund_tx_gas_limit() -> U256 {
        U256::from(100_000)
    }

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

    fn get_offset(name: &str, placeholder: &str) -> Offset {
        let contract = Self::compile_template_to_hex();

        let re_match = Regex::new(placeholder)
            .expect("Could not create regex")
            .find(contract.as_str())
            .expect("Could not find placeholder in hex code");
        Offset::new(
            name.into(),
            re_match.start() / 2,
            re_match.end() / 2,
            placeholder.len() / 2,
        )
    }

    pub fn get_all_offsets() -> Vec<Offset> {
        let refund_timestamp =
            Self::get_offset("Refund timestamp", Self::REFUND_TIMESTAMP_PLACEHOLDER);
        let redeem_address = Self::get_offset("Redeem Address\t", Self::REDEEM_ADDRESS_PLACEHOLDER);
        let refund_address = Self::get_offset("Refund Address\t", Self::REFUND_ADDRESS_PLACEHOLDER);
        let secret_hash = Self::get_offset("Secret Hash\t\t", Self::SECRET_HASH_PLACEHOLDER);
        let amount = Self::get_offset("Amount\t\t\t", Self::AMOUNT_PLACEHOLDER);
        let token_contract_address =
            Self::get_offset("Token Contract\t", Self::TOKEN_CONTRACT_ADDRESS_PLACEHOLDER);

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

impl Htlc for Erc20Htlc {
    fn compile_to_hex(&self) -> ByteCode {
        let refund_timestamp = format!("{:0>8x}", u32::from(self.refund_timestamp));
        let redeem_address = format!("{:x}", self.redeem_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let token_contract_address = format!("{:x}", self.token_contract_address);
        let amount = format!("{:0>64}", format!("{:x}", self.amount));

        let contract_code = Self::compile_template_to_hex()
            .replace(Self::REFUND_TIMESTAMP_PLACEHOLDER, &refund_timestamp)
            .replace(Self::REDEEM_ADDRESS_PLACEHOLDER, &redeem_address)
            .replace(Self::REFUND_ADDRESS_PLACEHOLDER, &refund_address)
            .replace(Self::SECRET_HASH_PLACEHOLDER, &secret_hash)
            .replace(Self::AMOUNT_PLACEHOLDER, &amount)
            .replace(
                Self::TOKEN_CONTRACT_ADDRESS_PLACEHOLDER,
                &token_contract_address,
            );

        ByteCode(contract_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rfc003::{secret_hash::SecretHash, timestamp::Timestamp};
    use std::str::FromStr;

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = Erc20Htlc::new(
            Timestamp::from(3000000),
            Address::default(),
            Address::default(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Address::default(),
            U256::from(100),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            Erc20Htlc::CONTRACT_CODE_TEMPLATE.len() + Erc20Htlc::DEPLOY_HEADER_TEMPLATE.len(),
            "HTLC is the same length as template plus deploy code"
        );
    }

    #[test]
    fn computes_funding_tx_payload_correctly() {
        let htlc = Erc20Htlc::new(
            Timestamp::from(2000000000),
            Address::default(),
            Address::default(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Address::default(),
            U256::from(100),
        );

        let htlc_hex = htlc.funding_tx_payload(Address::from(*base16!(
            "B97048628DB6B661D4C2AA833E95DBE1A905B280"
        )));
        let expected_bytes = base16!("A9059CBB000000000000000000000000B97048628DB6B661D4C2AA833E95DBE1A905B2800000000000000000000000000000000000000000000000000000000000000064");

        assert_eq!(htlc_hex, Bytes(expected_bytes.to_vec()));
    }
}
