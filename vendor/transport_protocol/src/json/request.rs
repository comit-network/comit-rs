use api::{self, IntoFrame};
use json;
use serde::{de::DeserializeOwned, ser::Serialize};
use serde_json::{self, Value as JsonValue};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize)]
pub struct Request {
    #[serde(rename = "type")]
    _type: String,
    headers: HashMap<String, JsonValue>,
    body: JsonValue,
}

impl Request {
    pub fn new(_type: String, headers: HashMap<String, JsonValue>, body: JsonValue) -> Self {
        Request {
            _type,
            headers,
            body,
        }
    }

    pub fn from_headers_and_body<H: Serialize, B: Serialize>(
        _type: String,
        headers: H,
        body: B,
    ) -> Result<Self, serde_json::Error> {
        let headers_json = serde_json::to_value(headers)?;
        let mut headers_hashmap = HashMap::new();

        match headers_json {
            JsonValue::Object(map) => {
                for (k, v) in map {
                    let _ = headers_hashmap.insert(k, v);
                }
            }
            _ => unreachable!(),
        }

        Ok(Request::new(
            _type,
            headers_hashmap,
            serde_json::to_value(body)?,
        ))
    }

    pub fn get_header<H: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<Result<H, serde_json::Error>> {
        self.headers
            .get(key)
            .map(|header| H::deserialize(header.clone()))
    }

    pub fn get_body<B: DeserializeOwned>(&self) -> Result<B, api::BodyError> {
        if self.body.is_null() {
            return Err(api::BodyError::Missing);
        }

        B::deserialize(self.body.clone()).or(Err(api::BodyError::Invalid))
    }
}

impl IntoFrame<json::Frame> for Request {
    fn into_frame(self, id: u32) -> json::Frame {
        // Serializing Request should never fail because its members are just Strings and JsonValues
        let payload = serde_json::to_value(self).unwrap();

        json::Frame::new("REQUEST".into(), id, payload)
    }
}
