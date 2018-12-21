use crate::{
    api::{self, IntoFrame},
    json::{self, frame::Frame},
};
use serde::{
    de::{self, Deserialize, DeserializeOwned, Deserializer, MapAccess, Visitor},
    Serialize,
};
use serde_json::{self, Map, Value as JsonValue};
use std::{collections::HashMap, fmt};

#[derive(Serialize, Debug, PartialEq)]
pub struct Response {
    status: api::Status,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    headers: HashMap<String, JsonValue>,
    #[serde(skip_serializing_if = "JsonValue::is_null")]
    #[serde(default)]
    body: JsonValue,
    #[serde(skip_serializing, skip_deserializing)]
    serialization_failure: bool,
}

impl Response {
    pub fn new(status: api::Status) -> Self {
        Response {
            status,
            headers: HashMap::new(),
            serialization_failure: false,
            body: JsonValue::Null,
        }
    }

    pub fn status(&self) -> &api::Status {
        &self.status
    }

    pub fn with_header<H: Serialize>(mut self, key: &str, header: H) -> Self {
        if self.serialization_failure {
            warn!("Not adding header because previous header or body failed to serialize. This request will not be sent!");
            return self;
        }

        match serde_json::to_value(header) {
            Ok(value) => {
                self.insert(key.to_string(), value);
            }
            Err(e) => {
                error!("Error during serialization of header: {:?}", e);
                self.serialization_failure = true;
            }
        }

        self
    }

    pub fn with_body<B: Serialize>(mut self, body: B) -> Self {
        if self.serialization_failure {
            warn!("Not adding body because previous header or body failed to serialize. This request will not be sent!");
            return self;
        }

        match serde_json::to_value(body) {
            Ok(value) => self.body = value,
            Err(e) => {
                error!("Error during serialization of body: {:?}", e);
                self.serialization_failure = true;
            }
        }

        self
    }

    fn insert<S: Into<String>>(&mut self, key: S, header: JsonValue) {
        self.headers.insert(key.into(), Self::compact_value(header));
    }

    fn compact_value(value: JsonValue) -> JsonValue {
        match value {
            JsonValue::Object(map) => {
                if map.len() == 1 {
                    let (_key, inner_value) = map.into_iter().next().unwrap();
                    inner_value
                } else {
                    JsonValue::Object(map)
                }
            }
            _ => value,
        }
    }

    pub fn get_body(&self) -> &JsonValue {
        &self.body
    }

    pub fn get_header<H: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<Result<H, serde_json::Error>> {
        self.headers
            .get(key)
            .map(|header| H::deserialize(header.clone()))
    }
}

impl IntoFrame<Frame> for Response {
    fn into_frame(self, id: u32) -> Frame {
        // Serializing Response should never fail because its members are just Strings
        // and JsonValues
        let payload = serde_json::to_value(self).unwrap();

        Frame::new("RESPONSE".into(), id, payload)
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Headers,
            Status,
            Body,
        };

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("`status` or `headers` or `body`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "headers" => Ok(Field::Headers),
                            "status" => Ok(Field::Status),
                            "body" => Ok(Field::Body),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ResponseVisitor;

        impl<'de> Visitor<'de> for ResponseVisitor {
            type Value = Response;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct Response")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Response, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut headers = None;
                let mut status = None;
                let mut body = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Status => {
                            if status.is_some() {
                                return Err(de::Error::duplicate_field("status"));
                            }
                            status = Some(map.next_value()?);
                        }
                        Field::Headers => {
                            if headers.is_some() {
                                return Err(de::Error::duplicate_field("headers"));
                            }

                            let _headers: Map<String, JsonValue> = map.next_value()?;
                            let mut normalized_headers = HashMap::new();

                            for (key, value) in _headers.into_iter() {
                                let value = json::normalize_compact_header(value);

                                normalized_headers.insert(key, value);
                            }

                            headers = Some(normalized_headers);
                        }
                        Field::Body => {
                            if body.is_some() {
                                return Err(de::Error::duplicate_field("body"));
                            }
                            body = Some(map.next_value()?);
                        }
                    }
                }
                let status = status.ok_or_else(|| de::Error::missing_field("status"))?;
                let status = serde_json::from_value(status).map_err(de::Error::custom)?;
                let headers = headers.unwrap_or_default();
                let body = body.unwrap_or(JsonValue::Null);

                let response = Response {
                    status,
                    headers,
                    serialization_failure: false,
                    body,
                };
                debug!("Map deserialized as response: {:?}", response);

                Ok(response)
            }
        }

        const FIELDS: &[&str] = &["headers", "status", "body"];
        deserializer.deserialize_struct("Response", FIELDS, ResponseVisitor)
    }
}
