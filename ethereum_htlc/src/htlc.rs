use chrono;
use common_types::secret::SecretHash;
use ethereum_support::{Address, Bytes, U256};
use hex;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Debug)]
pub struct Htlc {
    expiry_timestamp: SystemTime,
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

impl Into<SystemTime> for EpochOffset {
    fn into(self) -> SystemTime {
        SystemTime::now() + self.0
    }
}

impl Htlc {
    const CONTRACT_CODE_TEMPLATE: &'static str = include_str!("../contract.asm.hex");
    const EXPIRY_TIMESTAMP_PLACEHOLDER: &'static str = "20000002";
    const SUCCESS_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";

    pub fn new<
        ExpiryTimestamp: Into<SystemTime>,
        RefundAddress: Into<Address>,
        SuccessAddress: Into<Address>,
        Hash: Into<SecretHash>,
    >(
        expiry_timestamp: ExpiryTimestamp,
        refund_address: RefundAddress,
        success_address: SuccessAddress,
        secret_hash: Hash,
    ) -> Self {
        let expiry_timestamp: SystemTime = expiry_timestamp.into();
        let refund_address: Address = refund_address.into();
        let success_address: Address = success_address.into();
        let secret_hash = secret_hash.into();

        debug!(
            "Created HTLC with secret hash {:?} for address {}. At the earliest of {}, {} can reclaim the funds.",
            secret_hash,
            success_address,
            chrono::DateTime::<chrono::Utc>::from(expiry_timestamp).to_rfc3339(),
            refund_address
        );

        Htlc {
            expiry_timestamp,
            refund_address,
            success_address,
            secret_hash,
        }
    }

    pub fn compile_to_hex(&self) -> ByteCode {
        let expiry_timestamp = self.expiry_timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expiry_timestamp = format!("{:x}", expiry_timestamp);
        let success_address = format!("{:x}", self.success_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let contract_code = Self::CONTRACT_CODE_TEMPLATE
            .to_string()
            .replace(Self::EXPIRY_TIMESTAMP_PLACEHOLDER, &expiry_timestamp)
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

        // Variables
        let memory_start_address = "00";
        let deploy_code_length = "0C";
        let code_length = format!("{:2X}", code.len() / 2);
        let code_length = code_length.as_str();

        let op_codes = &[
            PUSH1,
            code_length,
            PUSH1,
            deploy_code_length,
            PUSH1,
            memory_start_address,
            CODE_COPY,
            PUSH1,
            code_length,
            PUSH1,
            memory_start_address,
            RETURN,
        ];

        op_codes.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = Htlc::new(
            SystemTime::now(),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            ).unwrap(),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            Htlc::CONTRACT_CODE_TEMPLATE.len() + 12 * 2,
            "HTLC is the same length as template plus deploy code"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_no_longer_contain_placeholders() {
        let htlc = Htlc::new(
            SystemTime::now(),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ).unwrap(),
        );

        let compiled_code = htlc.compile_to_hex().0;

        assert!(!compiled_code.contains(Htlc::EXPIRY_TIMESTAMP_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::SUCCESS_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::REFUND_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(Htlc::SECRET_HASH_PLACEHOLDER));
    }

    #[test]
    fn should_generate_correct_deploy_header() {
        let htlc = Htlc::new(
            SystemTime::now(),
            Address::new(),
            Address::new(),
            SecretHash::from_str("").unwrap(),
        );
        let deploy_header =
            htlc.generate_deploy_header("731000000000000000000000000000000000000001ff");

        assert_eq!(&deploy_header, "6016600C60003960166000F3");
    }
}
