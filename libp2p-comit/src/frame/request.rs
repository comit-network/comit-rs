use crate::{
    frame::header::{Header, Headers},
    Frame, FrameKind,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct UnvalidatedInboundRequest {
    #[serde(flatten)]
    inner: Request,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidatedInboundRequest {
    #[serde(flatten)]
    inner: Request,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboundRequest {
    #[serde(flatten)]
    inner: Request,
}

impl ValidatedInboundRequest {
    pub fn request_type(&self) -> &str {
        self.inner.request_type.as_str()
    }

    pub fn header(&self, key: &str) -> Option<&Header> {
        self.inner.headers.get(key)
    }

    pub fn take_header(&mut self, key: &str) -> Option<Header> {
        self.inner.headers.take(key)
    }

    pub fn take_body_as<B>(self) -> Result<B, serde_json::Error>
    where
        B: DeserializeOwned,
    {
        self.inner.take_body_as()
    }
}

impl OutboundRequest {
    pub fn new<T>(request_type: T) -> Self
    where
        T: Into<String>,
    {
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

impl UnvalidatedInboundRequest {
    pub fn request_type(&self) -> &str {
        self.inner.request_type.as_str()
    }

    pub fn ensure_no_unknown_mandatory_headers(
        self,
        known_headers: &HashSet<String>,
    ) -> Result<ValidatedInboundRequest, UnknownMandatoryHeaders> {
        let (parsed_headers, unknown_mandatory_headers) = self.inner.headers.into_iter().fold(
            (Headers::default(), UnknownMandatoryHeaders::default()),
            |(parsed_headers, mut unknown_headers), (key, header)| {
                if key.must_understand && !known_headers.contains(&key.value) {
                    unknown_headers.add(key.value);

                    (parsed_headers, unknown_headers)
                } else {
                    let parsed_headers = parsed_headers.with_header(&key.value, header);

                    (parsed_headers, unknown_headers)
                }
            },
        );

        if !unknown_mandatory_headers.is_empty() {
            return Err(unknown_mandatory_headers);
        }

        Ok(ValidatedInboundRequest {
            inner: Request {
                request_type: self.inner.request_type,
                headers: parsed_headers,
                body: self.inner.body,
            },
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct UnknownMandatoryHeaders(HashSet<String>);

impl UnknownMandatoryHeaders {
    pub fn add(&mut self, header_key: String) {
        self.0.insert(header_key);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
    pub fn take_body_as<B>(self) -> Result<B, serde_json::Error>
    where
        B: DeserializeOwned,
    {
        B::deserialize(self.body)
    }
}

impl From<OutboundRequest> for Frame {
    fn from(r: OutboundRequest) -> Frame {
        let payload = serialize(r);
        Frame::new(FrameKind::Request, payload)
    }
}

fn serialize(r: OutboundRequest) -> JsonValue {
    // Serializing and OutboundRequest should never fail because its
    // members are just Strings and JsonValues.
    serde_json::to_value(r).unwrap()
}
