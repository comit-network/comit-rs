use proptest::prelude::*;

pub mod ethereum {
    use super::*;
    use crate::ethereum::*;

    prop_compose! {
        pub fn address()(bytes in any::<[u8; 20]>()) -> Address {
            bytes.into()
        }
    }

    prop_compose! {
        pub fn hash()(bytes in any::<[u8; 32]>()) -> Hash {
            bytes.into()
        }
    }
}
