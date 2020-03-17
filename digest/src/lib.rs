pub use digest_macro_derive::DigestMacro;
pub use hex;

pub use multihash;
use multihash::Multihash;

pub fn digest(bytes: &[u8]) -> Multihash {
    multihash::Sha3_256::digest(bytes)
}

pub trait Digest {
    fn digest(self) -> Multihash;
}

pub trait FieldDigest {
    fn field_digest(self, prefix: Vec<u8>) -> Multihash;
}

impl<T> FieldDigest for T
where
    T: Digest,
{
    fn field_digest(self, prefix: Vec<u8>) -> Multihash {
        let mut bytes = prefix;
        let field_digest = self.digest();
        bytes.append(&mut field_digest.into_bytes());

        digest(&bytes)
    }
}

impl FieldDigest for String {
    fn field_digest(self, prefix: Vec<u8>) -> Multihash {
        let mut bytes = prefix;
        // String::into_bytes return the bytes for UTF-8 encoding
        let mut value = self.into_bytes();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

impl FieldDigest for Vec<u8> {
    fn field_digest(mut self, prefix: Vec<u8>) -> Multihash {
        let mut bytes = prefix;
        bytes.append(&mut self);

        digest(&self)
    }
}
