use multihash::Multihash;

const SEPARATOR: &[u8; 1] = b":";

pub fn digest(bytes: &[u8]) -> Multihash {
    // Time the tests and take fastest hash?
    multihash::Sha3_256::digest(bytes)
}

pub trait DigestRoot {
    fn digest_root(self) -> Multihash;
}

pub trait DigestField {
    fn digest_field(self, field_name: String) -> Multihash;
}

impl DigestField for String {
    fn digest_field(self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        let mut value = self.into_bytes();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

impl DigestField for Vec<u8> {
    fn digest_field(mut self, field_name: String) -> Multihash {
        let mut bytes = field_name.into_bytes();
        let mut separator = SEPARATOR.to_vec();
        bytes.append(&mut separator);
        bytes.append(&mut self);

        digest(&self)
    }
}
