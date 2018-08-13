use common_types::secret::SecretHash;
use ethereum_support::{Address, Bytes, U256};
use hex;
use std::time::Duration;

#[derive(Debug)]
pub struct Htlc {
    expiry_offset: U256,
    refund_address: Address,
    success_address: Address,
    secret_hash: SecretHash,
}

pub struct ByteCode(String);

impl Into<Bytes> for ByteCode {
    fn into(self) -> Bytes {
        Bytes(hex::decode(self.0).unwrap())
    }
}

#[derive(Clone)]
pub struct EpochOffset(Duration);

impl EpochOffset {
    pub fn hours(hours: u64) -> Self {
        EpochOffset(Duration::from_secs(60 * 60 * hours))
    }

    pub fn to_u256(&self) -> U256 {
        U256::from(self.0.as_secs())
    }
}

impl From<EpochOffset> for U256 {
    fn from(offset: EpochOffset) -> Self {
        offset.to_u256()
    }
}

impl Htlc {
    const CONTRACT_CODE_TEMPLATE: &'static str = include_str!("../contract.asm.hex");
    const EXPIRY_OFFSET_PLACEHOLDER: &'static str = "20000002";
    const SUCCESS_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";
    const DEPLOY_CODE_LENGTH: usize = 21;

    pub fn new<
        ExpiryOffset: Into<U256>,
        RefundAddress: Into<Address>,
        SuccessAddress: Into<Address>,
        Hash: Into<SecretHash>,
    >(
        expiry_offset: ExpiryOffset,
        refund_address: RefundAddress,
        success_address: SuccessAddress,
        secret_hash: Hash,
    ) -> Self {
        let expiry_offset: U256 = expiry_offset.into();
        let refund_address: Address = refund_address.into();
        let success_address: Address = success_address.into();
        let secret_hash = secret_hash.into();

        debug!(
            "Created HTLC with secret hash {:?} for address {}. After {}s, {} can reclaim the funds.",
            secret_hash,
            success_address,
            expiry_offset,
            refund_address
        );

        Htlc {
            expiry_offset,
            refund_address,
            success_address,
            secret_hash,
        }
    }

    fn int_to_hex_left_padded(int: U256) -> String {
        let hex = format!("{:x}", int);

        format!("{:0>8}", hex)
    }

    pub fn compile_to_hex(&self) -> ByteCode {
        let expiry_offset = Self::int_to_hex_left_padded(self.expiry_offset);
        let success_address = format!("{:x}", self.success_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let contract_code = Self::CONTRACT_CODE_TEMPLATE
            .to_string()
            .replace(Self::EXPIRY_OFFSET_PLACEHOLDER, &expiry_offset)
            .replace(Self::SUCCESS_ADDRESS_PLACEHOLDER, &success_address)
            .replace(Self::REFUND_ADDRESS_PLACEHOLDER, &refund_address)
            .replace(Self::SECRET_HASH_PLACEHOLDER, &secret_hash);

        let deploy_header = self.generate_deploy_header(&contract_code);

        debug!("Final contract code: {}", &contract_code);
        debug!("Deploy header: {}", &deploy_header);

        let deployable_contract = deploy_header + &contract_code;

        debug!("Deployable contract: {}", &deployable_contract);

        ByteCode(deployable_contract)
    }

    /// Don't touch this unless you know what you are doing!
    #[allow(non_snake_case)]
    fn generate_deploy_header(&self, code: &str) -> String {
        // Necessary op codes
        let PUSH1 = "60";
        let CODE_COPY = "39";
        let RETURN = "F3";
        let TIMESTAMP = "42";
        let MSTORE = "52";
        let MSTORE8 = "53";

        // Variables
        let deploy_timestamp_memory_start_address = "1B";
        let contract_code_memory_start_address = "20";
        let timestamp_start_address = "00";
        let push4_opcode_value = "63";

        let deploy_timestamp_length = 1 + 4; // PUSH4 + TIMESTAMP
        let code_length = code.len() / 2; // In hex, each byte is two chars

        let code_length_as_hex = format!("{:2X}", code_length);
        let size_of_code_to_copy_in_hex = format!("{:2X}", code_length - deploy_timestamp_length);

        let program_counter_code_start_address =
            format!("{:2X}", Htlc::DEPLOY_CODE_LENGTH + deploy_timestamp_length);

        let op_codes = &[
            TIMESTAMP,
            PUSH1,
            timestamp_start_address,
            MSTORE,
            PUSH1,
            push4_opcode_value,
            PUSH1,
            deploy_timestamp_memory_start_address,
            MSTORE8,
            PUSH1,
            size_of_code_to_copy_in_hex.as_str(),
            PUSH1,
            program_counter_code_start_address.as_str(),
            PUSH1,
            contract_code_memory_start_address,
            CODE_COPY,
            PUSH1,
            code_length_as_hex.as_str(),
            PUSH1,
            deploy_timestamp_memory_start_address,
            RETURN,
        ];

        debug_assert_eq!(op_codes.len(), Htlc::DEPLOY_CODE_LENGTH);

        op_codes.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn should_correctly_pad_integer() {
        let padded_string = Htlc::int_to_hex_left_padded(U256::from(32));

        assert_eq!(&padded_string, "00000020");
    }

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = Htlc::new(
            U256::from(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            ).unwrap(),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            Htlc::CONTRACT_CODE_TEMPLATE.len() + Htlc::DEPLOY_CODE_LENGTH * 2,
            "HTLC is the same length as template plus deploy code"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_no_longer_contain_placeholders() {
        let htlc = Htlc::new(
            U256::from(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ).unwrap(),
        );

        let compiled_code = htlc.compile_to_hex().0;

        assert!(!compiled_code.contains(Htlc::EXPIRY_OFFSET_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::SUCCESS_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::REFUND_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::SECRET_HASH_PLACEHOLDER));
    }

    #[test]
    fn should_generate_correct_deploy_header() {
        let htlc = Htlc::new(
            U256::from(100),
            Address::new(),
            Address::new(),
            SecretHash::from_str("").unwrap(),
        );
        let deploy_header = htlc.generate_deploy_header(Htlc::CONTRACT_CODE_TEMPLATE);

        assert_eq!(&deploy_header, "426000526063601B53607D601A6020396082601BF3");
    }
}
