use ethereum_support::{web3::types::Bytes, Address, U256};
use swap_protocols::rfc003::{
    ethereum::{ByteCode, Htlc, Seconds},
    SecretHash,
};

#[derive(Debug, Clone)]
pub struct Erc20Htlc {
    refund_timeout: Seconds,
    refund_address: Address,
    success_address: Address,
    secret_hash: SecretHash,
    token_contract_address: Address,
    amount: U256,
}

impl Erc20Htlc {
    const CONTRACT_CODE_TEMPLATE: &'static str =
        include_str!("./contract_templates/out/erc20_contract.asm.hex");
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";
    const REFUND_TIMEOUT_PLACEHOLDER: &'static str = "20000002";
    const SUCCESS_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const AMOUNT_PLACEHOLDER: &'static str =
        "5000000000000000000000000000000000000000000000000000000000000005";
    const TOKEN_CONTRACT_ADDRESS_PLACEHOLDER: &'static str =
        "6000000000000000000000000000000000000006";

    const DEPLOY_HEADER_TEMPLATE: &'static str =
        include_str!("./contract_templates/out/erc20_deploy_header.asm.hex");
    const CONTRACT_START_POSITION_PLACEHOLDER: &'static str = "1001";
    const CONTRACT_LENGTH_PLACEHOLDER: &'static str = "2002";

    pub fn new(
        refund_timeout: Seconds,
        refund_address: Address,
        success_address: Address,
        secret_hash: SecretHash,
        token_contract_address: Address,
        amount: U256,
    ) -> Self {
        let htlc = Erc20Htlc {
            refund_timeout,
            refund_address,
            success_address,
            secret_hash,
            token_contract_address,
            amount,
        };

        debug!("Created new ERC20 HTLC for ethereum: {:#?}", htlc);

        htlc
    }

    /// Constructs the payload for funding an `Erc20` HTLC located at the given address.
    pub fn funding_tx_payload(&self, htlc_contract_address: Address) -> Bytes {
        let target_address = format!("{:0>64}", format!("{:x}", htlc_contract_address));
        let token_amount = format!("{:0>64}", format!("{:x}", self.amount));

        let data = format!("{}{}{}", "a9059cbb", target_address, token_amount);
        let hex_data = hex::decode(data).unwrap();

        Bytes::from(hex_data)
    }
}

impl Htlc for Erc20Htlc {
    fn compile_to_hex(&self) -> ByteCode {
        let refund_timeout = format!("{:0>8x}", self.refund_timeout.0);
        let success_address = format!("{:x}", self.success_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let token_contract_address = format!("{:x}", self.token_contract_address);
        let amount = format!("{:0>64}", format!("{:x}", self.amount));

        let contract_code = Self::CONTRACT_CODE_TEMPLATE
            .to_string()
            .replace(Self::REFUND_TIMEOUT_PLACEHOLDER, &refund_timeout)
            .replace(Self::SUCCESS_ADDRESS_PLACEHOLDER, &success_address)
            .replace(Self::REFUND_ADDRESS_PLACEHOLDER, &refund_address)
            .replace(Self::SECRET_HASH_PLACEHOLDER, &secret_hash)
            .replace(Self::AMOUNT_PLACEHOLDER, &amount)
            .replace(
                Self::TOKEN_CONTRACT_ADDRESS_PLACEHOLDER,
                &token_contract_address,
            );

        debug!("Final contract code: {}", &contract_code);

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

        debug!("Final contract code: {}", &contract_code);
        debug!("Deploy header: {}", &deploy_header);

        let deployable_contract = deploy_header + &contract_code;

        debug!("Deployable contract: {}", &deployable_contract);

        ByteCode(deployable_contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_support::{Address, U256};
    use std::str::FromStr;

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = Erc20Htlc::new(
            Seconds(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Address::new(),
            U256::from(100),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            Erc20Htlc::CONTRACT_CODE_TEMPLATE.len() + Erc20Htlc::DEPLOY_HEADER_TEMPLATE.len(),
            "HTLC is the same length as template plus deploy code"
        );
    }

}
