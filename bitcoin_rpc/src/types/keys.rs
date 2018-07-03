use bitcoin::util::privkey::Privkey;
use serde::de;
use serde::export::fmt;
use serde::{Deserializer, Serializer};
use std::str::FromStr;

#[derive(Deserialize, PartialEq, Serialize)]
pub struct PrivateKey {
    #[serde(deserialize_with = "deserialize_wif")]
    #[serde(serialize_with = "serialize_wif")]
    pub privkey: Privkey,
}

// wif = Wallet Import Format. Base58 format use by bitcoind
//fn serialize_wif<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
fn serialize_wif<S>(v: &Privkey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(v.to_string().as_str())
}

fn deserialize_wif<'de, D>(deserializer: D) -> Result<Privkey, <D as Deserializer<'de>>::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'vde> de::Visitor<'vde> for Visitor {
        type Value = Privkey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            formatter.write_str("a Wallet-Import-Format encoded value")
        }

        fn visit_str<E>(self, v: &str) -> Result<Privkey, E>
        where
            E: de::Error,
        {
            Privkey::from_str(v).map_err(|err| E::custom(format!("{}", err)))
        }
    }

    deserializer.deserialize_str(Visitor)
}

mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_private_key() {
        let private_key = PrivateKey {
            privkey: Privkey::from_str("cQ1DDxScq1rsYDdCUBywawwNVWTMwnLzCKCwGndC6MgdNtKPQ5Hz")
                .unwrap(),
        };

        let se_private_key = serde_json::to_string(&private_key).unwrap();
        let de_private_key = serde_json::from_str::<PrivateKey>(se_private_key.as_str()).unwrap();

        assert_eq!(
            private_key.privkey.secret_key(),
            de_private_key.privkey.secret_key()
        );
    }
}
