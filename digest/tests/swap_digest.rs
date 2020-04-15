use digest::{field_digest, Digest, Hash, IntoDigestInput};
// This is to ensure that it works even if other macros are used on the same
// struct
use serde::Serialize;

#[derive(Serialize)]
struct MyString(String);

impl IntoDigestInput for MyString {
    fn into_digest_input(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl From<&str> for MyString {
    fn from(str: &str) -> MyString {
        MyString(str.to_owned())
    }
}

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
struct MultihashSha256(::multihash::Multihash);

impl digest::Hash for MultihashSha256 {
    fn hash(bytes: &[u8]) -> Self {
        Self(multihash::Sha3_256::digest(bytes))
    }
}

impl IntoDigestInput for MultihashSha256 {
    fn into_digest_input(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

#[derive(Digest, Serialize)]
#[serde(rename_all = "lowercase")]
#[digest(hash = "MultihashSha256")]
struct DoubleFieldStruct {
    #[digest(prefix = "0011")]
    #[serde(alias = "foooo")]
    foo: MyString,
    #[digest(prefix = "FFAA")]
    bar: MyString,
    #[digest(ignore)]
    ignore: MyString,
}

struct OtherDoubleFieldStruct {
    bar: MyString,
    foo: MyString,
}

impl Digest for OtherDoubleFieldStruct {
    type Hash = MultihashSha256;

    fn digest(self) -> Self::Hash {
        let mut digests = vec![];
        let foo_digest = field_digest::<_, Self::Hash>(self.foo, [0x00u8, 0x11u8].to_vec());
        digests.push(foo_digest);
        let bar_digest = field_digest::<_, Self::Hash>(self.bar, [0xFFu8, 0xAAu8].to_vec());
        digests.push(bar_digest);

        digests.sort();

        let res = digests.into_iter().fold(vec![], |mut res, digest| {
            res.append(&mut digest.0.into_bytes());
            res
        });

        Self::Hash::hash(&res)
    }
}

#[derive(Digest, Serialize)]
#[digest(hash = "MultihashSha256")]
#[serde(rename_all = "lowercase")]
enum Enum {
    #[serde(alias = "foooo")]
    #[digest(prefix = "0011")]
    Foo,
    #[digest(prefix = "0E0F")]
    Bar,
}

enum OtherEnum {
    Foo,
    Bar,
}

impl Digest for OtherEnum {
    type Hash = MultihashSha256;
    fn digest(self) -> Self::Hash {
        let bytes = match self {
            OtherEnum::Foo => vec![0x00u8, 0x11u8],
            OtherEnum::Bar => vec![0x00u8, 0x11u8],
        };

        Self::Hash::hash(&bytes)
    }
}

#[test]
fn given_same_strings_return_same_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "simple string".into();

    assert_eq!(
        field_digest::<_, MultihashSha256>(str1, "foo".into()),
        field_digest::<_, MultihashSha256>(str2, "foo".into())
    )
}

#[test]
fn given_same_strings_different_names_return_diff_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "simple string".into();

    assert_ne!(
        field_digest::<_, MultihashSha256>(str1, "foo".into()),
        field_digest::<_, MultihashSha256>(str2, "bar".into())
    )
}

#[test]
fn given_different_strings_return_different_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "longer string".into();

    assert_ne!(
        field_digest::<_, MultihashSha256>(str1, "foo".into()),
        field_digest::<_, MultihashSha256>(str2, "foo".into())
    )
}

#[test]
fn given_same_double_field_struct_return_same_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
        ignore: "Does not matter".into(),
    };
    let struct2 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
        ignore: "So it can be whatever you want".into(),
    };

    assert_eq!(struct1.digest(), struct2.digest())
}

#[test]
fn given_different_double_field_struct_return_different_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
        ignore: "Does not matter".into(),
    };
    let struct2 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "different field".into(),
        ignore: "So it can be whatever you want".into(),
    };

    assert_ne!(struct1.digest(), struct2.digest())
}

#[test]
fn given_two_double_field_struct_with_same_data_return_same_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "foo field".into(),
        bar: "bar field".into(),
        ignore: "this field does not matter".into(),
    };
    let struct2 = OtherDoubleFieldStruct {
        bar: "bar field".into(),
        foo: "foo field".into(),
    };

    assert_eq!(struct1.digest(), struct2.digest())
}

#[test]
fn given_two_enums_with_same_bytes_per_variant_return_same_multihash() {
    let enum1 = Enum::Foo;
    let enum2 = OtherEnum::Foo;

    assert_eq!(enum1.digest(), enum2.digest())
}

#[test]
fn given_two_enums_with_differnt_variant_return_different_multihash() {
    let enum1 = Enum::Foo;
    let enum2 = OtherEnum::Bar;

    assert_eq!(enum1.digest(), enum2.digest())
}
