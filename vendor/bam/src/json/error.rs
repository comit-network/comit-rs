use crate::{
    api::IntoFrame,
    json::{frame::Frame, header::Header, FrameType},
};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ErrorType {
    #[serde(rename = "unknown-frame-type")]
    UnknownFrameType,
    #[serde(rename = "malformed-frame")]
    MalformedFrame,
    #[serde(rename = "unknown-request-type")]
    UnknownRequestType,
    #[serde(rename = "unknown-mandatory-header")]
    UnknownMandatoryHeader,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    error_type: ErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Header>,
}

impl Error {
    pub fn new(error_type: ErrorType) -> Self {
        Error {
            error_type,
            details: None,
        }
    }

    pub fn error_type(&self) -> &ErrorType {
        &self.error_type
    }

    pub fn with_details(self, details: Header) -> Self {
        Error {
            details: Some(details),
            ..self
        }
    }

    pub fn details(&self) -> &Option<Header> {
        &self.details
    }
}

impl IntoFrame<Frame> for Error {
    fn into_frame(self) -> Frame {
        // Serializing Error should never fail because its members are just Strings
        // and JsonValues
        let payload = serde_json::to_value(self).unwrap();

        Frame::new(FrameType::Error, payload)
    }
}
