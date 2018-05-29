use Address;
use SecretHash;

pub struct Htlc {
    expiry_timestamp: u32,
    refund_address: Address,
    success_address: Address,
    secret_hash: SecretHash,
}

impl Htlc {
    const TEMPLATE: &'static str = include_str!("../contract.asm.hex");
    const EXPIRY: &'static str = "20000002";
    const REDEEM: &'static str = "3000000000000000000000000000000000000003";
    const REFUND: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";

    pub fn new(
        expiry_timestamp: u32,
        refund_address: Address,
        success_address: Address,
        secret_hash: SecretHash,
    ) -> Self {
        Htlc {
            expiry_timestamp,
            refund_address,
            success_address,
            secret_hash,
        }
    }

    pub fn compile_to_hex(&self) -> String {
        Self::TEMPLATE
            .to_string()
            .replace(
                Self::EXPIRY,
                format!("{:x}", self.expiry_timestamp).as_str(),
            )
            .replace(Self::REDEEM, format!("{:x}", self.success_address).as_str())
            .replace(Self::REFUND, format!("{:x}", self.refund_address).as_str())
            .replace(
                Self::SECRET_HASH,
                format!("{:x}", self.secret_hash).as_str(),
            )
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

}
