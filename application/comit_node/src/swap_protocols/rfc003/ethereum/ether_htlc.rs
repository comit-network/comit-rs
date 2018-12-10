use ethereum_support::{Address, Bytes, U256};
use swap_protocols::rfc003::{
    ethereum::{ByteCode, Htlc, Seconds},
    SecretHash,
};

#[derive(Debug)]
pub struct EtherHtlc {
    refund_timeout: Seconds,
    refund_address: Address,
    redeem_address: Address,
    secret_hash: SecretHash,
}

impl EtherHtlc {
    const CONTRACT_CODE_TEMPLATE: &'static str =
        include_str!("./contract_templates/out/ether_contract.asm.hex");
    const REFUND_TIMEOUT_PLACEHOLDER: &'static str = "20000002";
    const REDEEM_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";

    const DEPLOY_HEADER_TEMPLATE: &'static str =
        include_str!("./contract_templates/out/ether_deploy_header.asm.hex");
    const CONTRACT_START_POSITION_PLACEHOLDER: &'static str = "1001";
    const CONTRACT_LENGTH_PLACEHOLDER: &'static str = "2002";

    pub fn new(
        refund_timeout: Seconds,
        refund_address: Address,
        redeem_address: Address,
        secret_hash: SecretHash,
    ) -> Self {
        let htlc = EtherHtlc {
            refund_timeout,
            refund_address,
            redeem_address,
            secret_hash,
        };

        trace!("Created new HTLC for ethereum: {:#?}", htlc);

        htlc
    }

    pub fn deployment_gas_limit(&self) -> U256 {
        let bytes: Bytes = self.compile_to_hex().into();
        let n_bytes = bytes.0.len();
        let gas_per_byte = 200;

        U256::from(50_000 + n_bytes * gas_per_byte)
    }

    pub fn tx_gas_limit() -> U256 {
        U256::from(100_000)
    }
}

impl Htlc for EtherHtlc {
    fn compile_to_hex(&self) -> ByteCode {
        let refund_timeout = format!("{:0>8x}", self.refund_timeout.0);
        let redeem_address = format!("{:x}", self.redeem_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let contract_code = Self::CONTRACT_CODE_TEMPLATE
            .to_string()
            .replace(Self::REFUND_TIMEOUT_PLACEHOLDER, &refund_timeout)
            .replace(Self::REDEEM_ADDRESS_PLACEHOLDER, &redeem_address)
            .replace(Self::REFUND_ADDRESS_PLACEHOLDER, &refund_address)
            .replace(Self::SECRET_HASH_PLACEHOLDER, &secret_hash);

        let code_length = contract_code.len() / 2; // In hex, each byte is two chars

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

        trace!("Final contract code: {}", &contract_code);
        trace!("Deploy header: {}", &deploy_header);

        let deployable_contract = deploy_header + &contract_code;

        trace!("Deployable contract: {}", &deployable_contract);

        ByteCode(deployable_contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = EtherHtlc::new(
            Seconds(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            EtherHtlc::CONTRACT_CODE_TEMPLATE.len() + EtherHtlc::DEPLOY_HEADER_TEMPLATE.len(),
            "HTLC is the same length as template plus deploy code"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_no_longer_contain_placeholders() {
        let htlc = EtherHtlc::new(
            Seconds(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );

        let compiled_code = htlc.compile_to_hex().0;

        assert!(!compiled_code.contains(EtherHtlc::REFUND_TIMEOUT_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::REDEEM_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::REFUND_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::SECRET_HASH_PLACEHOLDER));
    }
}
