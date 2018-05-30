use Address;
use SecretHash;
use hex;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use web3;

pub struct Htlc {
    expiry_timestamp: u32,
    refund_address: Address,
    success_address: Address,
    secret_hash: SecretHash,
}

// TODO: Create IntoAddress and IntoSecretHash trait

pub trait IntoAddress {
    fn into_address(self) -> Address;
}

pub trait IntoSecretHash {
    fn into_secret_hash(self) -> SecretHash;
}

impl IntoAddress for Address {
    fn into_address(self) -> Address {
        self
    }
}

impl IntoSecretHash for SecretHash {
    fn into_secret_hash(self) -> SecretHash {
        self
    }
}

impl IntoAddress for &'static str {
    fn into_address(self) -> Address {
        let hex = hex::decode(self).expect("Given string is not hex-encoded");
        Address::from_slice(hex.as_ref())
    }
}

impl IntoSecretHash for &'static str {
    fn into_secret_hash(self) -> SecretHash {
        let hex = hex::decode(self).expect("Given string is not hex-encoded");
        SecretHash::from_slice(hex.as_ref())
    }
}

pub trait IntoExpiryTimestamp {
    fn into_timestamp(self) -> u32;
}

impl IntoExpiryTimestamp for Duration {
    fn into_timestamp(self) -> u32 {
        self.as_secs() as u32
    }
}

pub struct ByteCode(String);

impl ByteCode {
    pub fn into_bytes(self) -> web3::types::Bytes {
        web3::types::Bytes(hex::decode(self.0).unwrap())
    }
}

pub struct EpochOffset(Duration);

impl EpochOffset {
    pub fn hours(hours: u64) -> Self {
        EpochOffset(Duration::from_secs(60 * 60 * hours))
    }
}

impl IntoExpiryTimestamp for EpochOffset {
    fn into_timestamp(self) -> u32 {
        let unix_epoch_duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        (unix_epoch_duration + self.0).into_timestamp()
    }
}

impl Htlc {
    const TEMPLATE: &'static str = include_str!("../contract.asm.hex");
    const EXPIRY: &'static str = "20000002";
    const SUCCESS: &'static str = "3000000000000000000000000000000000000003";
    const REFUND: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";

    pub fn new<
        ExpiryTimestamp: IntoExpiryTimestamp,
        RefundAddress: IntoAddress,
        SuccessAddress: IntoAddress,
        SecretHash: IntoSecretHash,
    >(
        expiry_timestamp: ExpiryTimestamp,
        refund_address: RefundAddress,
        success_address: SuccessAddress,
        secret_hash: SecretHash,
    ) -> Self {
        Htlc {
            expiry_timestamp: expiry_timestamp.into_timestamp(),
            refund_address: refund_address.into_address(),
            success_address: success_address.into_address(),
            secret_hash: secret_hash.into_secret_hash(),
        }
    }

    pub fn compile_to_hex(&self) -> ByteCode {
        let contract_code = Self::TEMPLATE
            .to_string()
            .replace(
                Self::EXPIRY,
                format!("{:x}", self.expiry_timestamp).as_str(),
            )
            .replace(
                Self::SUCCESS,
                format!("{:x}", self.success_address).as_str(),
            )
            .replace(Self::REFUND, format!("{:x}", self.refund_address).as_str())
            .replace(
                Self::SECRET_HASH,
                format!("{:x}", self.secret_hash).as_str(),
            );

        let code = format!(
            "{}{}",
            self.generate_deploy_header(&contract_code),
            contract_code
        );

        ByteCode(code)
    }

    /// Don't touch this unless you know what you are doing!
    #[allow(non_snake_case)]
    fn generate_deploy_header(&self, code: &str) -> String {
        let PUSH1 = "60";
        let memory_start_address = "00";
        let deploy_code_length = "0C";
        let CODE_COPY = "39";
        let code_length = format!("{:2X}", code.len() / 2);
        let code_length = code_length.as_str();
        let RETURN = "F3";

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

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let epoch = 1527559350;
        let epoch_hex = "5b0cb4b6";
        let htlc = Htlc::new(epoch, Address::new(), Address::new(), SecretHash::new());
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.len(),
            Htlc::TEMPLATE.len(),
            "HTLC is the same length as template"
        );
        assert!(
            htlc_hex.contains(epoch_hex),
            "The epoch_hex exists in output"
        );
    }

    #[test]
    fn should_prepend_deploy_code_to_contract() {
        let epoch = 1527559350;
        let epoch_hex = "5b0cb4b6";
        let htlc = Htlc::new(epoch, Address::new(), Address::new(), SecretHash::new());
        let htlc_hex = htlc.compile_to_hex();

        assert_eq!(&htlc_hex[0..24], "6069600C60003960696000F3");
    }

    #[test]
    fn deploy_code_length_should_be_12_opcodes() {
        let epoch = 1527559350;
        let epoch_hex = "5b0cb4b6";
        let htlc = Htlc::new(epoch, Address::new(), Address::new(), SecretHash::new());
        let deploy_header = htlc.generate_deploy_header("");

        // If this test fails, you need to rethink the deploy code. The deploy code needs to know about its number of opcodes, otherwise it cannot deploy the contract.
        assert_eq!(deploy_header.len(), 24);
    }
}
