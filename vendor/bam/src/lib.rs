#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

mod api;
#[macro_use]
pub mod json;

pub use crate::api::*;

use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};

pub trait IntoFrame<F> {
    fn into_frame(self) -> F;
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Frame {
    #[serde(rename = "type")]
    pub frame_type: FrameType,
    pub payload: JsonValue,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum FrameType {
    Request,
    Response,
    Error,
}

impl Frame {
    pub fn new(frame_type: FrameType, payload: JsonValue) -> Self {
        Self {
            frame_type,
            payload,
        }
    }
}
