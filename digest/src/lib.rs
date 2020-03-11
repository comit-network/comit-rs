pub use multihash;
use multihash::Multihash;

const SEPARATOR: &[u8; 1] = b":";

pub fn digest(bytes: &[u8]) -> Multihash {
    multihash::Sha3_256::digest(bytes)
}

pub trait RootDigest {
    fn digest_root(self) -> Multihash;
}

pub trait FieldDigest {
    fn digest_field(self, field_name: String) -> Multihash;
}

impl FieldDigest for String {
    fn digest_field(self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        let mut value = self.into_bytes();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

impl FieldDigest for Vec<u8> {
    fn digest_field(mut self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        bytes.append(&mut self);

        digest(&self)
    }
}
