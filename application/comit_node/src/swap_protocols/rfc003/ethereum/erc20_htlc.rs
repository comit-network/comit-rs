use ethereum_support::{web3::types::Bytes, Address, U256};
use swap_protocols::rfc003::{
    ethereum::{ByteCode, Htlc, Seconds},
    SecretHash,
};

#[derive(Debug)]
pub struct Erc20Htlc {
    refund_timeout: Seconds,
    refund_address: Address,
    success_address: Address,
    secret_hash: SecretHash,
    htlc_contract_address: Address,
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

    pub fn new(
        refund_timeout: Seconds,
        refund_address: Address,
        success_address: Address,
        secret_hash: SecretHash,
        htlc_contract_address: Address,
        token_contract_address: Address,
        amount: U256,
    ) -> Self {
        let htlc = Erc20Htlc {
            refund_timeout,
            refund_address,
            success_address,
            secret_hash,
            htlc_contract_address,
            token_contract_address,
            amount,
        };

        debug!("Created new ERC20 HTLC for ethereum: {:#?}", htlc);

        htlc
    }

    pub fn transfer_call(&self) -> Bytes {
        unimplemented!()
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

        ByteCode(contract_code)
    }
}
