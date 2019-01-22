use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum CompactOrExtended {
    Compact {
        value: serde_json::Value,
    },
    Extended {
        value: serde_json::Value,
        #[serde(default)]
        parameters: BTreeMap<String, serde_json::Value>,
    },
}

impl CompactOrExtended {
    fn value(&self) -> &serde_json::Value {
        match self {
            CompactOrExtended::Compact { value } | CompactOrExtended::Extended { value, .. } => {
                value
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
pub struct Header {
    #[serde(flatten)]
    inner: CompactOrExtended,
}

impl Header {
    pub fn value<V: DeserializeOwned>(&self) -> Result<V, serde_json::Error> {
        serde_json::from_value(self.inner.value().clone())
    }

    /// Returns the parameter with the provided key converted into the type `P`.
    ///
    /// If the parameter doesn't exist, `Null` is passed to the conversion
    /// (which will very likely fail). TODO: Is this a good idea? :D
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

    pub fn with_json_value(value: serde_json::Value) -> Header {
        Header {
            inner: CompactOrExtended::Compact { value },
        }
    }

    pub fn with_str_value(value: &str) -> Header {
        Header {
            inner: CompactOrExtended::Compact {
                value: serde_json::Value::String(value.to_string()),
            },
        }
    }

    pub fn with_value<V: Serialize>(value: V) -> Result<Header, serde_json::Error> {
        Ok(Header {
            inner: CompactOrExtended::Compact {
                value: serde_json::to_value(value)?,
            },
        })
    }

    pub fn with_parameter<P: Serialize>(
        self,
        key: &'static str,
        parameter: P,
    ) -> Result<Header, serde_json::Error> {
        let (value, mut parameters) = match self.inner {
            CompactOrExtended::Compact { value } => (value, BTreeMap::new()),
            CompactOrExtended::Extended { value, parameters } => (value, parameters),
        };

        parameters.insert(String::from(key), serde_json::to_value(&parameter)?);

        Ok(Header {
            inner: CompactOrExtended::Extended { value, parameters },
        })
    }
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Headers {
    #[serde(flatten)]
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
