use crate::{seed::Seed, swap_protocols::rfc003::Secret};
use bitcoin::secp256k1::SecretKey;

pub trait SecretSource: Send + Sync + 'static {
    fn secret(&self) -> Secret;
    fn secp256k1_redeem(&self) -> SecretKey;
    fn secp256k1_refund(&self) -> SecretKey;
}

impl SecretSource for Seed {
    fn secret(&self) -> Secret {
        self.sha256_with_seed(&[b"SECRET"]).into()
    }

    fn secp256k1_redeem(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REDEEM"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }

    fn secp256k1_refund(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REFUND"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }
}
