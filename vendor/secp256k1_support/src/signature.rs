pub use secp256k1::Message;
pub use secp256k1::SecretKey;
pub use secp256k1::Signature;

pub trait DerSerializableSignature {
    fn serialize_signature_der(&self) -> Vec<u8>;
}

impl DerSerializableSignature for Signature {
    fn serialize_signature_der(&self) -> Vec<u8> {
        self.serialize_der(&*super::SECP)
    }
}
