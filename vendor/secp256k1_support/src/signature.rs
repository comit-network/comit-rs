use secp256k1;
pub use secp256k1::{recovery, Message, SecretKey, Signature};

pub trait DerSerializableSignature {
    fn serialize_signature_der(&self) -> Vec<u8>;
}

impl DerSerializableSignature for Signature {
    fn serialize_signature_der(&self) -> Vec<u8> {
        self.serialize_der().to_vec()
    }
}

pub trait RecoverableSignature {
    fn serialize_compact(&self) -> (recovery::RecoveryId, [u8; 64]);
}

impl RecoverableSignature for recovery::RecoverableSignature {
    fn serialize_compact(&self) -> (recovery::RecoveryId, [u8; 64]) {
        recovery::RecoverableSignature::serialize_compact(&self)
    }
}
