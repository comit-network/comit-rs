use digest::{digest, FieldDigest, RootDigest, RootDigestMacro};

use digest::multihash::Multihash;

struct NewType(String);

impl RootDigest for NewType {
    fn root_digest(self) -> Multihash {
        self.0.field_digest("0".into())
    }
}

struct SingleFieldStruct {
    field: String,
}

impl RootDigest for SingleFieldStruct {
    fn root_digest(self) -> Multihash {
        self.field.field_digest("field".into())
    }
}

#[derive(RootDigestMacro)]
struct DoubleFieldStruct {
    #[digest_bytes = "0011"]
    foo: String,
    #[digest_bytes = "FFAA"]
    bar: String,
}

struct OtherStruct {
    bar: String,
    foo: String,
}

impl RootDigest for OtherStruct {
    fn root_digest(self) -> Multihash {
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

#[derive(RootDigestMacro)]
enum Enum {
    #[digest_bytes = "0011"]
    Foo,
    #[digest_bytes = "0E0F"]
    Bar,
}

enum OtherEnum {
    Foo,
    Bar,
}

impl RootDigest for OtherEnum {
    fn root_digest(self) -> Multihash {
        let bytes = match self {
            OtherEnum::Foo => digest(vec![0x00u8, 0x11u8]),
            OtherEnum::Bar => digest(vec![0x0Eu8, 0x0Fu8]),
        };

        digest(&bytes)
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
fn given_same_newtypes_return_same_multihash() {
    let new_type1 = NewType("simple string".into());
    let new_type2 = NewType("simple string".into());

    assert_eq!(new_type1.root_digest(), new_type2.root_digest())
}

#[test]
fn given_different_newtypes_return_different_multihash() {
    let new_type1 = NewType("simple string".into());
    let new_type2 = NewType("longer string.".into());

    assert_ne!(new_type1.root_digest(), new_type2.root_digest())
}

#[test]
fn given_same_single_field_struct_return_same_multihash() {
    let struct1 = SingleFieldStruct {
        field: "foo".into(),
    };
    let struct2 = SingleFieldStruct {
        field: "foo".into(),
    };

    assert_eq!(struct1.root_digest(), struct2.root_digest())
}

#[test]
fn given_single_field_struct_and_new_type_with_same_inner_return_different_multihash() {
    let single_field_struct = SingleFieldStruct {
        field: "foo".into(),
    };
    let new_type = NewType("foo".into());

    assert_ne!(single_field_struct.root_digest(), new_type.root_digest())
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

    assert_eq!(struct1.root_digest(), struct2.root_digest())
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

    assert_ne!(struct1.root_digest(), struct2.root_digest())
}

#[test]
fn given_two_double_field_struct_with_same_data_return_same_multihash() {
    let struct1 = DoubleFieldStruct {
        foo: "foo field".into(),
        bar: "bar field".into(),
    };
    let struct2 = OtherStruct {
        bar: "bar field".into(),
        foo: "foo field".into(),
    };

    assert_eq!(struct1.root_digest(), struct2.root_digest())
}

#[test]
fn given_two_enums_with_same_bytes_per_variant_return_same_multihash() {
    let enum1 = Enum::Foo;
    let enum2 = OtherEnum::Foo;

    assert_eq!(enum1.root_digest(), enum2.root_digest())
}
