/// This crate brings two traits: `Digest` and `IntoDigestInput`
///
/// `Digest` should be implemented on data structures using the `Digest`
/// derive macro.
/// The attribute `digest_prefix` must be applied on each field. It plays the
/// role of an identifier to ensure that fields with same data but different
/// meaning do not result to the same digest.
///
/// Data types within the data structure should implement `IntoDigestInput`,
/// this allows you to control how the data is transformed to a byte array.
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
/// 1. For each field in the struct:
///     a. Concatenate `digest_prefix` + `self.into_digest_input`,
///     b. Hash the result.
/// 2. Lexically order the resulting field digests,
/// 3. Concatenate the list,
/// 4. Hash the result.
///
/// For unit variants of enums, only the prefix as input to the hash function.
/// Note that Nested structures are not supported.
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

pub fn field_digest<T: IntoDigestInput>(field: T, prefix: Vec<u8>) -> Multihash {
    let mut bytes = prefix;
    let mut value = field.into_digest_input();
    bytes.append(&mut value);

    digest(&bytes)
}
