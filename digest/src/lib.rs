/// This crate brings two traits: `Digest` and `ToDigestInput`
///
/// `Digest` should be implemented on data structures using the `Digest`
/// derive macro.
/// The attribute `digest_prefix` must be applied on each field. It plays the
/// role of an identifier to ensure that fields with same data but different
/// meaning do not result to the same digest.
///
/// Data types within the data structure should implement `ToDigestInput`,
/// this allows you to control how the data is transformed to a byte array.
///
/// ```
/// use digest::{Digest, Hash, ToDigestInput};
/// // multihash from libp2p for example: `use libp2p::multihash`
/// use multihash;
///
/// // Define a hash type. Hash will need to be sorted as per the digest algo described
/// // After this code block so it needs to implement `Eq`, `Ord`, `PartialEq`, `PartialOrd`
/// #[derive(Eq, Ord, PartialEq, PartialOrd)]
/// struct MyHash(multihash::Multihash);
///
/// // Define a hash function
/// impl Hash for MyHash {
///     fn hash(bytes: &[u8]) -> Self {
///         Self(multihash::Sha3_256::digest(bytes))
///     }
/// }
///
/// // Define how to get a byte array from the hash type
/// impl ToDigestInput for MyHash {
///     fn to_digest_input(&self) -> Vec<u8> {
///         self.0.clone().into_bytes()
///     }
/// }
///
/// // Define types for the field of the struct you want to digest
/// struct MyType(Vec<u8>);
///
/// // And implement `ToDigestInput` for them
/// impl ToDigestInput for MyType {
///     fn to_digest_input(&self) -> Vec<u8> {
///         self.0.clone()
///     }
/// }
///
/// #[derive(Digest)]
/// #[digest(hash = "MyHash")]
/// struct MyStruct {
///     #[digest(prefix = "00AA")]
///     foo: MyType,
///     #[digest(prefix = "1122")]
///     bar: MyType,
/// }
/// ```
///
/// The digest algorithm goes as follow:
/// 1. For each field in the struct:
///     a. Concatenate `digest_prefix` + `self.to_digest_input`,
///     b. Hash the result.
/// 2. Lexically order the resulting field digests,
/// 3. Concatenate the list,
/// 4. Hash the result.
///
/// For unit variants of enums, only the prefix as input to the hash function.
/// Note that Nested structures are not supported.
pub use digest_macro_derive::Digest;
pub use hex;

pub trait Digest {
    type Hash: Hash + ToDigestInput;
    fn digest(&self) -> Self::Hash;
}

pub trait Hash {
    fn hash(bytes: &[u8]) -> Self;
}

pub trait ToDigestInput {
    fn to_digest_input(&self) -> Vec<u8>;
}

pub fn field_digest<T, H>(field: &T, prefix: Vec<u8>) -> H
where
    T: ToDigestInput,
    H: Hash,
{
    let mut bytes = prefix;
    let mut value = field.to_digest_input();
    bytes.append(&mut value);

    H::hash(&bytes)
}
