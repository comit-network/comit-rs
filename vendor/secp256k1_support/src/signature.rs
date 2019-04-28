use secp256k1;
pub use secp256k1::{Message, RecoveryId, SecretKey, Signature};

pub trait DerSerializableSignature {
    fn serialize_signature_der(&self) -> Vec<u8>;
}

impl DerSerializableSignature for Signature {
    fn serialize_signature_der(&self) -> Vec<u8> {
        self.serialize_der()
    }
}

pub trait RecoverableSignature {
    fn serialize_compact(&self) -> (RecoveryId, [u8; 64]);
}

impl RecoverableSignature for secp256k1::RecoverableSignature {
    fn serialize_compact(&self) -> (RecoveryId, [u8; 64]) {
        secp256k1::RecoverableSignature::serialize_compact(&self)
    }
}
