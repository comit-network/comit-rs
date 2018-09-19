use api::{self, IntoFrame};
use json::frame::Frame;
use serde::Serialize;
use serde_json::{self, Value as JsonValue};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

    pub fn body(&self) -> &JsonValue {
        &self.body
    }
}

impl IntoFrame<Frame> for Response {
    fn into_frame(self, id: u32) -> Frame {
        // Serializing Response should never fail because its members are just Strings and JsonValues
        let payload = serde_json::to_value(self).unwrap();

        Frame::new("RESPONSE".into(), id, payload)
    }
}
