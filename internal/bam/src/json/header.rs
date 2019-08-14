use serde::{
    de::{DeserializeOwned, Deserializer},
    Deserialize, Serialize,
};
use std::collections::{BTreeMap, HashMap};

fn deserialize_compact_value<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Object(_) => Err(serde::de::Error::custom(
            "header value must not be an object in compact form",
        )),
        serde_json::Value::Null => Err(serde::de::Error::custom(
            "header value must not be null in compact form",
        )),
        other => Ok(other),
    }
}

fn deserialize_extended_value<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Err(serde::de::Error::custom(
            "header value must not be null in extended form",
        )),
        other => Ok(other),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum CompactOrExtended {
    Extended {
        #[serde(deserialize_with = "deserialize_extended_value")]
        value: serde_json::Value,
        #[serde(default)]
        parameters: BTreeMap<String, serde_json::Value>,
    },
    #[serde(deserialize_with = "deserialize_compact_value")]
    Compact(serde_json::Value),
}

impl CompactOrExtended {
    fn value(&self) -> serde_json::Value {
        match self {
            CompactOrExtended::Compact(value) | CompactOrExtended::Extended { value, .. } => {
                value.clone()
            }
        }
    }

    fn take_parameter(&mut self, key: &'static str) -> Option<serde_json::Value> {
        match self {
            CompactOrExtended::Compact { .. } => None,
            CompactOrExtended::Extended { parameters, .. } => parameters.remove(key),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Header {
    inner: CompactOrExtended,
}

impl Header {
    pub fn value<V: DeserializeOwned>(&self) -> Result<V, serde_json::Error> {
        serde_json::from_value(self.inner.value())
    }

    /// Returns the parameter with the provided key converted into the type `P`.
    ///
    /// If the parameter doesn't exist, `Null` is passed to the conversion. This
    /// is equivalent to the parameter actually being present in the JSON but
    /// the `value` being null. This allows to change the otherwise cumbersome
    /// return type of `Option<Result<P, serde_json::Error>>` to `Result<P,
    /// serde_json::Error>`. The caller has to handle conversion errors anyway.
    pub fn take_parameter<P: DeserializeOwned>(
        &mut self,
        key: &'static str,
    ) -> Result<P, serde_json::Error> {
        let parameter = self
            .inner
            .take_parameter(key)
            .unwrap_or(serde_json::Value::Null);

        serde_json::from_value(parameter)
    }

    /// Returns the parameter with the provided key converted into the type `P`
    /// or `P::default()` if the parameter is not present.
    pub fn take_parameter_or_default<P: DeserializeOwned + Default>(
        &mut self,
        key: &'static str,
    ) -> Result<P, serde_json::Error> {
        self.inner
            .take_parameter(key)
            .map(serde_json::from_value)
            .unwrap_or_else(|| Ok(P::default()))
    }

    pub fn with_json_value(value: serde_json::Value) -> Header {
        Header {
            inner: CompactOrExtended::Compact(value),
        }
    }

    pub fn with_str_value(value: &str) -> Header {
        Header {
            inner: CompactOrExtended::Compact(serde_json::Value::String(value.to_string())),
        }
    }

    pub fn with_value<V: Serialize>(value: V) -> Result<Header, serde_json::Error> {
        Ok(Header {
            inner: CompactOrExtended::Compact(serde_json::to_value(value)?),
        })
    }

    pub fn with_parameter<P: Serialize>(
        self,
        key: &'static str,
        parameter: P,
    ) -> Result<Header, serde_json::Error> {
        let (value, mut parameters) = match self.inner {
            CompactOrExtended::Compact(value) => (value, BTreeMap::new()),
            CompactOrExtended::Extended { value, parameters } => (value, parameters),
        };

        parameters.insert(String::from(key), serde_json::to_value(&parameter)?);

        Ok(Header {
            inner: CompactOrExtended::Extended { value, parameters },
        })
    }
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers {
    inner: HashMap<String, Header>,
}

pub struct HeaderKey {
    pub value: String,
    pub must_understand: bool,
}

impl Headers {
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn with_header(mut self, key: &str, header: Header) -> Self {
        self.inner.insert(key.to_string(), header);

        self
    }

    pub fn get(&self, key: &str) -> Option<&Header> {
        self.inner.get(key)
    }

    pub fn take(&mut self, key: &str) -> Option<Header> {
        self.inner.remove(key)
    }

    pub fn into_iter(self) -> impl Iterator<Item = (HeaderKey, Header)> {
        self.inner.into_iter().map(|(mut key, value)| {
            let non_mandatory = key.starts_with('_');

            if non_mandatory {
                key.remove(0);
            }

            let must_understand = !non_mandatory;

            (
                HeaderKey {
                    value: key,
                    must_understand,
                },
                value,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::json::{header::Headers, Header};
    use spectral::prelude::*;

    #[test]
    fn can_deserialize_compact_header() {
        let json = r#"{"key": "HELLO WORLD"}"#;

        let headers = serde_json::from_str(json);

        assert_that(&headers).is_ok_containing(
            Headers::default().with_header("key", Header::with_str_value("HELLO WORLD")),
        );
    }

    #[test]
    fn can_deserialize_extended_header() {
        let json = r#"{"key": {
            "value": "HELLO WORLD",
            "parameters": {"foo": "bar"}
        }}"#;

        let headers = serde_json::from_str(json);

        assert_that(&headers).is_ok_containing(
            Headers::default().with_header(
                "key",
                Header::with_str_value("HELLO WORLD")
                    .with_parameter("foo", "bar")
                    .unwrap(),
            ),
        );
    }

    #[test]
    fn can_serialize_compact_header() {
        let headers = Headers::default().with_header("key", Header::with_str_value("HELLO WORLD"));

        let expected_json = r#"{"key":"HELLO WORLD"}"#;

        let actual_json = serde_json::to_string(&headers);

        assert_that(&actual_json).is_ok_containing(expected_json.to_string());
    }

    #[test]
    fn can_serialize_extended_header() {
        let headers = Headers::default().with_header(
            "key",
            Header::with_str_value("HELLO WORLD")
                .with_parameter("foo", "bar")
                .unwrap(),
        );

        let expected_json = r#"{"key":{"value":"HELLO WORLD","parameters":{"foo":"bar"}}}"#;

        let actual_json = serde_json::to_string(&headers);

        assert_that(&actual_json).is_ok_containing(expected_json.to_string());
    }
}
