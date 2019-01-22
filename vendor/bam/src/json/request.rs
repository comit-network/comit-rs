use crate::{
    api::IntoFrame,
    json::{
        self,
        frame::{HeaderErrorKind, HeaderErrors},
        header::{Header, Headers},
    },
};
use serde::de::DeserializeOwned;
use serde_json::{self, Value as JsonValue};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct UnvalidatedIncomingRequest {
    #[serde(flatten)]
    inner: Request,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidatedIncomingRequest {
    #[serde(flatten)]
    inner: Request,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutgoingRequest {
    #[serde(flatten)]
    inner: Request,
}

impl ValidatedIncomingRequest {
    pub fn header(&self, key: &str) -> Option<&Header> {
        self.inner.headers.get(key)
    }

    pub fn take_header(&mut self, key: &str) -> Option<Header> {
        self.inner.headers.take(key)
    }

    pub fn take_body_as<B: DeserializeOwned>(self) -> Result<B, serde_json::Error> {
        self.inner.take_body_as()
    }
}

impl OutgoingRequest {
    pub fn new<T: Into<String>>(request_type: T) -> Self {
        Self {
            inner: Request {
                request_type: request_type.into(),
                headers: Headers::default(),
                body: serde_json::Value::Null,
            },
        }
    }

    pub fn with_header(self, key: &str, header: Header) -> Self {
        let request = self.inner;

        Self {
            inner: Request {
                headers: request.headers.with_header(key, header),
                ..request
            },
        }
    }

    pub fn with_body(self, body: JsonValue) -> Self {
        Self {
            inner: Request { body, ..self.inner },
        }
    }
}

impl UnvalidatedIncomingRequest {
    pub fn request_type(&self) -> &str {
        self.inner.request_type.as_str()
    }

    pub(crate) fn validate(
        self,
        known_headers: &HashSet<String>,
    ) -> Result<ValidatedIncomingRequest, HeaderErrors> {
        let (parsed_headers, errors) = self.inner.headers.into_iter().fold(
            (Headers::default(), HeaderErrors::new()),
            |(parsed_headers, mut errors), (key, header)| {
                if !known_headers.contains(&key.value) && key.must_understand {
                    errors.add_error(key.value.clone(), HeaderErrorKind::UnknownMandatoryHeader)
                }

                (parsed_headers.with_header(&key.value, header), errors)
            },
        );

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(ValidatedIncomingRequest {
            inner: Request {
                request_type: self.inner.request_type,
                headers: parsed_headers,
                body: self.inner.body,
            },
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Request {
    #[serde(rename = "type")]
    request_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Headers::is_empty")]
    headers: Headers,
    #[serde(default)]
    #[serde(skip_serializing_if = "JsonValue::is_null")]
    body: JsonValue,
}

impl Request {
    pub fn take_body_as<B: DeserializeOwned>(self) -> Result<B, serde_json::Error> {
        B::deserialize(self.body)
    }
}

impl IntoFrame<json::Frame> for OutgoingRequest {
    fn into_frame(self, id: u32) -> json::Frame {
        // Serializing Request should never fail because its members are just Strings
        // and JsonValues
        let payload = serde_json::to_value(self).unwrap();

        json::Frame::new("REQUEST".into(), id, payload)
    }
}
