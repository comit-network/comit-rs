use digest::{digest, Digest, DigestMacro, FieldDigest};

use digest::multihash::Multihash;

#[derive(DigestMacro)]
struct DoubleFieldStruct {
    #[digest_prefix = "0011"]
    foo: String,
    #[digest_prefix = "FFAA"]
    bar: String,
}

struct OtherDoubleFieldStruct {
    bar: String,
    foo: String,
}

impl Digest for OtherDoubleFieldStruct {
    fn digest(self) -> Multihash {
        let mut digests = vec![];
        let foo_digest = self.foo.field_digest([0x00u8, 0x11u8].to_vec());
        digests.push(foo_digest);
        let bar_digest = self.bar.field_digest([0xFFu8, 0xAAu8].to_vec());
        digests.push(bar_digest);

        digests.sort();

        let res = digests.into_iter().fold(vec![], |mut res, digest| {
            res.append(&mut digest.into_bytes());
            res
        });

        digest(&res)
    }
}

#[derive(DigestMacro)]
enum Enum {
    #[digest_prefix = "0011"]
    Foo,
    #[digest_prefix = "0E0F"]
    Bar,
}

#[allow(dead_code)]
enum OtherEnum {
    Foo,
    Bar,
}

impl Digest for OtherEnum {
    fn digest(self) -> Multihash {
        let bytes = match self {
            OtherEnum::Foo => vec![0x00u8, 0x11u8],
            OtherEnum::Bar => vec![0x00u8, 0x11u8],
        };

        digest(&bytes)
    }
}

#[derive(DigestMacro)]
struct NestedStruct {
    #[digest_prefix = "0011"]
    foo: String,
    #[digest_prefix = "AA00"]
    nest: DoubleFieldStruct,
}

struct OtherNestedStruct {
    foo: String,
    nest: OtherDoubleFieldStruct,
}

impl Digest for OtherNestedStruct {
    fn digest(self) -> Multihash {
        let mut digests = vec![];
        let foo_digest = self.foo.field_digest([0x00u8, 0x11u8].to_vec());
        digests.push(foo_digest);
        let nest_digest = self.nest.field_digest([0xAAu8, 0x00u8].to_vec());
        digests.push(nest_digest);

        digests.sort();

        let res = digests.into_iter().fold(vec![], |mut res, digest| {
            res.append(&mut digest.into_bytes());
            res
        });

        digest(&res)
    }
}

#[test]
fn given_same_strings_return_same_multihash() {
    let str1 = String::from("simple string");
    let str2 = String::from("simple string");

    assert_eq!(
        str1.field_digest("foo".into()),
        str2.field_digest("foo".into())
    )
}

#[test]
fn given_same_strings_different_names_return_diff_multihash() {
    let str1 = String::from("simple string");
    let str2 = String::from("simple string");

    assert_ne!(
        str1.field_digest("foo".into()),
        str2.field_digest("bar".into())
    )
}

#[test]
fn given_different_strings_return_different_multihash() {
    let str1 = String::from("simple string");
    let str2 = String::from("longer string.");

    assert_ne!(
        str1.field_digest("foo".into()),
        str2.field_digest("foo".into())
    )
}

#[test]
fn given_same_double_field_struct_return_same_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
    };
    let struct2 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
    };

    assert_eq!(struct1.digest(), struct2.digest())
}

#[test]
fn given_different_double_field_struct_return_different_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "second field".into(),
    };
    let struct2 = DoubleFieldStruct {
        foo: "first field".into(),
        bar: "different field".into(),
    };

    assert_ne!(struct1.digest(), struct2.digest())
}

#[test]
fn given_two_double_field_struct_with_same_data_return_same_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "foo field".into(),
        bar: "bar field".into(),
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
fn given_two_nested_structs_with_same_value_return_same_multihash() {
    let struct1 = NestedStruct {
        foo: "foo".to_string(),
        nest: DoubleFieldStruct {
            foo: "phou".to_string(),
            bar: "pub".to_string(),
        },
    };
    let struct2 = OtherNestedStruct {
        foo: "foo".to_string(),
        nest: OtherDoubleFieldStruct {
            foo: "phou".to_string(),
            bar: "pub".to_string(),
        },
    };

    assert_eq!(struct1.digest(), struct2.digest())
}
