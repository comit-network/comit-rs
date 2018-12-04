use http::Uri;
use serde::{de, export::fmt, Deserializer};
use std::str::FromStr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Uri, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Uri;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an url")
        }

        fn visit_str<E>(self, value: &str) -> Result<Uri, E>
        where
            E: de::Error,
        {
            Uri::from_str(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
