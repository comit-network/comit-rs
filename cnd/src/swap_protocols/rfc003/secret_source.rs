use crate::{seed::Seed, swap_protocols::rfc003::Secret};
use bitcoin_support::secp256k1_omni_context::KeyPair;

pub trait SecretSource: Send + Sync {
    fn secret(&self) -> Secret;
    fn secp256k1_redeem(&self) -> KeyPair;
    fn secp256k1_refund(&self) -> KeyPair;
}

impl SecretSource for Seed {
    fn secret(&self) -> Secret {
        self.sha256_with_seed(&[b"SECRET"]).into()
    }

    fn secp256k1_redeem(&self) -> KeyPair {
        KeyPair::from_secret_key_slice(
            crate::SECP.clone(),
            self.sha256_with_seed(&[b"REDEEM"]).as_ref(),
        )
        .expect("The probability of this happening is < 1 in 2^120")
    }

    fn secp256k1_refund(&self) -> KeyPair {
        KeyPair::from_secret_key_slice(
            crate::SECP.clone(),
            self.sha256_with_seed(&[b"REFUND"]).as_ref(),
        )
        .expect("The probability of this happening is < 1 in 2^120")
    }
}
