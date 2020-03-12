pub use digest_macro_derive::RootDigestMacro;
pub use hex;

pub use multihash;
use multihash::Multihash;

const SEPARATOR: u8 = 0u8;

pub fn digest(bytes: &[u8]) -> Multihash {
    multihash::Sha3_256::digest(bytes)
}

pub trait RootDigest {
    fn root_digest(self) -> Multihash;
}

pub trait FieldDigest {
    fn field_digest(self, suffix: Vec<u8>) -> Multihash;
}

impl FieldDigest for String {
    fn field_digest(self, suffix: Vec<u8>) -> Multihash {
        let mut bytes = suffix;
        let mut separator = [SEPARATOR].to_vec();
        bytes.append(&mut separator);
        // String::into_bytes return the bytes for UTF-8 encoding
        let mut value = self.into_bytes();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

impl FieldDigest for Vec<u8> {
    fn field_digest(mut self, suffix: Vec<u8>) -> Multihash {
        let mut bytes = suffix;
        let mut separator = [SEPARATOR].to_vec();
        bytes.append(&mut separator);
        bytes.append(&mut self);

        digest(&self)
    }
}
