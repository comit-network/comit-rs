pub use multihash;
use multihash::Multihash;

const SEPARATOR: &[u8; 1] = b":";

pub fn digest(bytes: &[u8]) -> Multihash {
    multihash::Sha3_256::digest(bytes)
}

pub trait RootDigest {
    fn root_digest(self) -> Multihash;
}

pub trait FieldDigest {
    fn field_digest(self, field_name: String) -> Multihash;
}

impl FieldDigest for String {
    fn field_digest(self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        let mut value = self.into_bytes();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

impl FieldDigest for Vec<u8> {
    fn field_digest(mut self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        bytes.append(&mut self);

        digest(&self)
    }
}
