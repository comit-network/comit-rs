use bitcoin::util::privkey::Privkey;
use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::export::fmt;
use serde::{Deserializer, Serializer};
use std::fmt as std_fmt;
use std::str::FromStr;

#[derive(PartialEq)]
pub struct RpcPrivateKey(Privkey);

impl Into<Privkey> for RpcPrivateKey {
    fn into(self) -> Privkey {
        self.0
    }
}

impl Serialize for RpcPrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for RpcPrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = RpcPrivateKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("a Wallet-Import-Format encoded value")
            }

            fn visit_str<E>(self, v: &str) -> Result<RpcPrivateKey, E>
            where
                E: de::Error,
            {
                let privkey = Privkey::from_str(v).map_err(|err| E::custom(format!("{}", err)))?;
                Ok(RpcPrivateKey(privkey))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl std_fmt::Debug for RpcPrivateKey {
    fn fmt(&self, f: &mut std_fmt::Formatter) -> std_fmt::Result {
        write!(f, "PrivateKey {{ #REDACTED# }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_private_key() {
        let private_key = RpcPrivateKey(
            Privkey::from_str("cQ1DDxScq1rsYDdCUBywawwNVWTMwnLzCKCwGndC6MgdNtKPQ5Hz").unwrap(),
        );

        let se_private_key = serde_json::to_string(&private_key).unwrap();
        let de_private_key =
            serde_json::from_str::<RpcPrivateKey>(se_private_key.as_str()).unwrap();

        let priv_key: Privkey = private_key.into();
        let de_priv_key: Privkey = de_private_key.into();

        assert_eq!(priv_key.secret_key(), de_priv_key.secret_key());
    }
}
