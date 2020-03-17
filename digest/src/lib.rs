pub use digest_macro_derive::Digest;
pub use hex;

pub use multihash;
use multihash::Multihash;

pub fn digest(bytes: &[u8]) -> Multihash {
    multihash::Sha3_256::digest(bytes)
}

pub trait Digest {
    fn digest(self) -> Multihash;
}

pub trait IntoDigestInput {
    fn into_digest_input(self) -> Vec<u8>;
}

#[doc(hidden)]
pub trait FieldDigest: private::Sealed {
    fn field_digest(self, prefix: Vec<u8>) -> Multihash;
}

impl<T> IntoDigestInput for T
where
    T: Digest,
{
    fn into_digest_input(self) -> Vec<u8> {
        let field_digest = self.digest();

        field_digest.into_bytes()
    }
}

impl<T> FieldDigest for T
where
    T: IntoDigestInput,
{
    fn field_digest(self, prefix: Vec<u8>) -> Multihash {
        let mut bytes = prefix;
        let mut value = self.into_digest_input();
        bytes.append(&mut value);

        digest(&bytes)
    }
}

mod private {
    pub trait Sealed {}

    impl<T> Sealed for T where T: super::IntoDigestInput {}
}
