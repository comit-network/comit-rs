use multihash::Multihash;

trait Digest {
    fn digest(&self) -> Multihash;
}

impl Digest for String {
    fn digest(&self) -> Multihash {
        let bytes = self.as_bytes();
        // Time the tests and take fastest hash?
        multihash::Sha3_256::digest(bytes)
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
}
