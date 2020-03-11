use multihash::Multihash;

fn digest(bytes: &[u8]) -> Multihash {
    // Time the tests and take fastest hash?
    multihash::Sha3_256::digest(bytes)
}

trait Digest {
    fn digest(&self) -> Multihash;
}

impl Digest for String {
    fn digest(&self) -> Multihash {
        let bytes = self.as_bytes();
        digest(bytes)
    }
}

impl Digest for Vec<u8> {
    fn digest(&self) -> Multihash {
        digest(&self)
    }
}

struct NewType(String);

impl Digest for NewType {
    fn digest(&self) -> Multihash {
        self.0.digest()
    }
}

struct SingleFieldStruct {
    field: String,
}

impl Digest for SingleFieldStruct {
    fn digest(&self) -> Multihash {
        let mut str = String::from("field: ");
        str += &self.field;
        str.digest()
    }
}

struct DoubleFieldStruct {
    foo: String,
    bar: String,
}

impl Digest for DoubleFieldStruct {
    fn digest(&self) -> Multihash {
        let mut foo = String::from("foo: ");
        foo += &self.foo;
        let foo = foo.digest();

        let mut bar = String::from("bar: ");
        bar += &self.bar;
        let bar = bar.digest();

        if foo < bar {
            let mut res = foo.into_bytes();
            res.append(&mut bar.into_bytes());
            res.digest()
        } else {
            let mut res = bar.into_bytes();
            res.append(&mut foo.into_bytes());
            res.digest()
        }
    }
}

struct OtherStruct {
    bar: String,
    foo: String,
}

impl Digest for OtherStruct {
    fn digest(&self) -> Multihash {
        let mut foo = String::from("foo: ");
        foo += &self.foo;
        let foo = foo.digest();

        let mut bar = String::from("bar: ");
        bar += &self.bar;
        let bar = bar.digest();

        if foo < bar {
            let mut res = foo.into_bytes();
            res.append(&mut bar.into_bytes());
            res.digest()
        } else {
            let mut res = bar.into_bytes();
            res.append(&mut foo.into_bytes());
            res.digest()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_same_strings_return_same_multihash() {
        let str1 = String::from("simple string");
        let str2 = String::from("simple string");

        assert_eq!(str1.digest(), str2.digest())
    }

    #[test]
    fn given_differemt_strings_return_different_multihash() {
        let str1 = String::from("simple string");
        let str2 = String::from("longer string.");

        assert_ne!(str1.digest(), str2.digest())
    }

    #[test]
    fn given_same_newtypes_return_same_multihash() {
        let new_type1 = NewType("simple string".into());
        let new_type2 = NewType("simple string".into());

        assert_eq!(new_type1.digest(), new_type2.digest())
    }

    #[test]
    fn given_different_newtypes_return_different_multihash() {
        let new_type1 = NewType("simple string".into());
        let new_type2 = NewType("longer string.".into());

        assert_ne!(new_type1.digest(), new_type2.digest())
    }

    #[test]
    fn given_same_single_field_struct_return_same_multihash() {
        let struct1 = SingleFieldStruct {
            field: "foo".into(),
        };
        let struct2 = SingleFieldStruct {
            field: "foo".into(),
        };

        assert_eq!(struct1.digest(), struct2.digest())
    }

    #[test]
    fn given_single_field_struct_and_new_type_with_same_inner_return_different_multihash() {
        let single_field_struct = SingleFieldStruct {
            field: "foo".into(),
        };
        let new_type = NewType("foo".into());

        assert_ne!(single_field_struct.digest(), new_type.digest())
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
        let struct2 = OtherStruct {
            bar: "bar field".into(),
            foo: "foo field".into(),
        };

        assert_eq!(struct1.digest(), struct2.digest())
    }
}
