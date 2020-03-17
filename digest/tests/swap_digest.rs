use digest::{digest, Digest, DigestMacro, FieldDigest, IntoDigestInput};

use digest::multihash::Multihash;

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

#[derive(DigestMacro)]
struct DoubleFieldStruct {
    #[digest_prefix = "0011"]
    foo: MyString,
    #[digest_prefix = "FFAA"]
    bar: MyString,
}

struct OtherDoubleFieldStruct {
    bar: MyString,
    foo: MyString,
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
    foo: MyString,
    #[digest_prefix = "AA00"]
    nest: DoubleFieldStruct,
}

struct OtherNestedStruct {
    foo: MyString,
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

#[derive(DigestMacro)]
enum NestedEnum {
    #[digest_prefix = "DEAD"]
    Foo,
    #[digest_prefix = "BEEF"]
    Bar(NestedStruct),
}

#[allow(dead_code)]
enum OtherNestedEnum {
    Foo,
    Bar(OtherNestedStruct),
}

impl Digest for OtherNestedEnum {
    fn digest(self) -> Multihash {
        let bytes = match self {
            OtherNestedEnum::Foo => vec![0xDEu8, 0xADu8],
            OtherNestedEnum::Bar(other_nested_struct) => {
                let mut bytes = vec![0xBEu8, 0xEFu8];
                bytes.append(&mut other_nested_struct.digest().into_bytes());
                bytes
            }
        };

        digest(&bytes)
    }
}

#[test]
fn given_same_strings_return_same_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "simple string".into();

    assert_eq!(
        str1.field_digest("foo".into()),
        str2.field_digest("foo".into())
    )
}

#[test]
fn given_same_strings_different_names_return_diff_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "simple string".into();

    assert_ne!(
        str1.field_digest("foo".into()),
        str2.field_digest("bar".into())
    )
}

#[test]
fn given_different_strings_return_different_multihash() {
    let str1: MyString = "simple string".into();
    let str2: MyString = "longer string".into();

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
fn given_two_enums_with_differnt_variant_return_different_multihash() {
    let enum1 = Enum::Foo;
    let enum2 = OtherEnum::Bar;

    assert_eq!(enum1.digest(), enum2.digest())
}

#[test]
fn given_two_nested_structs_with_same_value_return_same_multihash() {
    let struct1 = NestedStruct {
        foo: "foo".into(),
        nest: DoubleFieldStruct {
            foo: "phou".into(),
            bar: "pub".into(),
        },
    };
    let struct2 = OtherNestedStruct {
        foo: "foo".into(),
        nest: OtherDoubleFieldStruct {
            foo: "phou".into(),
            bar: "pub".into(),
        },
    };

    assert_eq!(struct1.digest(), struct2.digest())
}

#[test]
fn given_two_nested_structs_with_diff_value_return_diff_multihash() {
    let struct1 = NestedStruct {
        foo: "phou".into(),
        nest: DoubleFieldStruct {
            foo: "foo".into(),
            bar: "pub".into(),
        },
    };
    let struct2 = OtherNestedStruct {
        foo: "phou".into(),
        nest: OtherDoubleFieldStruct {
            foo: "foo".into(),
            bar: "pub".into(),
        },
    };

    assert_eq!(struct1.digest(), struct2.digest())
}

#[test]
fn given_two_nested_enums_with_same_value_return_same_multihash() {
    let enum1 = NestedEnum::Bar(NestedStruct {
        foo: "foo".into(),
        nest: DoubleFieldStruct {
            foo: "faa".into(),
            bar: "restaurant".into(),
        },
    });
    let enum2 = OtherNestedEnum::Bar(OtherNestedStruct {
        foo: "foo".into(),
        nest: OtherDoubleFieldStruct {
            foo: "faa".into(),
            bar: "restaurant".into(),
        },
    });

    assert_eq!(enum1.digest(), enum2.digest());
}
