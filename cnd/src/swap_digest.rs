#[cfg(test)]
mod tests {
    use digest::{digest, DigestField, DigestRoot};
    use digest_macro_derive::DigestRootMacro;

    use digest::multihash::Multihash;

    struct NewType(String);

    impl DigestRoot for NewType {
        fn digest_root(self) -> Multihash {
            self.0.digest_field("".into())
        }
    }

    struct SingleFieldStruct {
        field: String,
    }

    impl DigestRoot for SingleFieldStruct {
        fn digest_root(self) -> Multihash {
            self.field.digest_field("field".into())
        }
    }

    #[derive(DigestRootMacro)]
    struct DoubleFieldStruct {
        foo: String,
        bar: String,
    }

    struct OtherStruct {
        bar: String,
        foo: String,
    }

    impl DigestRoot for OtherStruct {
        fn digest_root(self) -> Multihash {
            let mut digests = vec![];
            let foo_digest = self.foo.digest_field("foo".into());
            digests.push(foo_digest);
            let bar_digest = self.bar.digest_field("bar".into());
            digests.push(bar_digest);

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
            str1.digest_field("foo".into()),
            str2.digest_field("foo".into())
        )
    }

    #[test]
    fn given_same_strings_different_names_return_diff_multihash() {
        let str1 = String::from("simple string");
        let str2 = String::from("simple string");

        assert_ne!(
            str1.digest_field("foo".into()),
            str2.digest_field("bar".into())
        )
    }

    #[test]
    fn given_different_strings_return_different_multihash() {
        let str1 = String::from("simple string");
        let str2 = String::from("longer string.");

        assert_ne!(
            str1.digest_field("foo".into()),
            str2.digest_field("foo".into())
        )
    }

    #[test]
    fn given_same_newtypes_return_same_multihash() {
        let new_type1 = NewType("simple string".into());
        let new_type2 = NewType("simple string".into());

        assert_eq!(new_type1.digest_root(), new_type2.digest_root())
    }

    #[test]
    fn given_different_newtypes_return_different_multihash() {
        let new_type1 = NewType("simple string".into());
        let new_type2 = NewType("longer string.".into());

        assert_ne!(new_type1.digest_root(), new_type2.digest_root())
    }

    #[test]
    fn given_same_single_field_struct_return_same_multihash() {
        let struct1 = SingleFieldStruct {
            field: "foo".into(),
        };
        let struct2 = SingleFieldStruct {
            field: "foo".into(),
        };

        assert_eq!(struct1.digest_root(), struct2.digest_root())
    }

    #[test]
    fn given_single_field_struct_and_new_type_with_same_inner_return_different_multihash() {
        let single_field_struct = SingleFieldStruct {
            field: "foo".into(),
        };
        let new_type = NewType("foo".into());

        assert_ne!(single_field_struct.digest_root(), new_type.digest_root())
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

        assert_eq!(struct1.digest_root(), struct2.digest_root())
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

        assert_ne!(struct1.digest_root(), struct2.digest_root())
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

        assert_eq!(struct1.digest_root(), struct2.digest_root())
    }
}
