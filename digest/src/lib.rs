/// This crate brings two traits: `Digest` and `FieldDigest`
///
/// `Digest` should be implemented on data structures using the `Digest`
/// derive macro.
/// The attribute `digest_prefix` must be applied on each field. It plays the
/// role of an identifier to ensure that fields with same data but different
/// meaning do not result to the same digest.
///
/// Elementary data types should implement `IntoDigestInput`, this allows you to
/// control how the data is transformed to a byte array.
///
/// ```
/// use digest::{Digest, IntoDigestInput};
///
/// struct MyType(Vec<u8>);
///
/// impl IntoDigestInput for MyType {
///     fn into_digest_input(self) -> Vec<u8> {
///         self.0
///     }
/// }
///
/// #[derive(Digest)]
/// struct MyStruct {
///     #[digest_prefix = "00AA"]
///     foo: MyType,
///     #[digest_prefix = "1122"]
///     bar: MyType,
/// }
/// ```
///
/// The digest algorithm goes as follow:
/// 1. Compute `field_digest(prefix)` for all fields of the struct,
/// 2. Lexically order the field digests,
/// 3. Concatenate the result,
/// 4. Hash the result.
///
/// The field digest algorithm goes as follow:
/// For elementary types:
///     1. Transform the data in a byte array (if there is
///   any data) using `IntoDigestInput` trait,
///     2. Concatenate prefix and the resulting byte array,
///     3. Hash the result.
/// For data structures:
///     1. Calculate the root digest of the struct,
///     2. Concatenate prefix and resulting root digest,
///     3. Hash the result.
///
/// For unit variants of enums, only the prefix as input to the hash function.
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
