use bitcoin::util::privkey::Privkey;
use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::export::fmt;
use serde::{Deserializer, Serializer};
use std::fmt as std_fmt;
use std::str::FromStr;

#[derive(PartialEq)]
pub struct PrivateKey(pub Privkey);

impl Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = PrivateKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("a Wallet-Import-Format encoded value")
            }

            fn visit_str<E>(self, v: &str) -> Result<PrivateKey, E>
            where
                E: de::Error,
            {
                let privkey = Privkey::from_str(v).map_err(|err| E::custom(format!("{}", err)))?;
                Ok(PrivateKey(privkey))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl std_fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std_fmt::Formatter) -> std_fmt::Result {
        write!(f, "PrivateKey {{ {} }}", self.0.to_string())
    }
}

mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn debug_private_key() {
        let private_key = PrivateKey(
            Privkey::from_str("cQ1DDxScq1rsYDdCUBywawwNVWTMwnLzCKCwGndC6MgdNtKPQ5Hz").unwrap(),
        );

        println!("{:?}", private_key);
    }

    #[test]
    fn serialize_private_key() {
        let private_key = PrivateKey(
            Privkey::from_str("cQ1DDxScq1rsYDdCUBywawwNVWTMwnLzCKCwGndC6MgdNtKPQ5Hz").unwrap(),
        );

        let se_private_key = serde_json::to_string(&private_key).unwrap();
        let de_private_key = serde_json::from_str::<PrivateKey>(se_private_key.as_str()).unwrap();

        assert_eq!(private_key.0.secret_key(), de_private_key.0.secret_key());
    }
}
