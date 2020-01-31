use crate::{
    seed::SwapSeed,
    swap_protocols::rfc003::{Secret, SecretHash},
};
use bitcoin::secp256k1::SecretKey;

pub trait DeriveIdentities: Send + Sync + 'static {
    fn derive_redeem_identity(&self) -> SecretKey;
    fn derive_refund_identity(&self) -> SecretKey;
}

impl DeriveIdentities for SwapSeed {
    fn derive_redeem_identity(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REDEEM"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }

    fn derive_refund_identity(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REFUND"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }
}

pub trait DeriveSecret: Send + Sync + 'static {
    fn derive_secret(&self) -> Secret;
}

impl DeriveSecret for SwapSeed {
    fn derive_secret(&self) -> Secret {
        self.sha256_with_seed(&[b"SECRET"]).into()
    }
}
