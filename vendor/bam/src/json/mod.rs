mod codec;
mod frame;
mod request;
mod response;
#[macro_use]
mod macros;
pub mod status;

pub use self::{codec::*, frame::*, request::*, response::*, status::*};

use serde_json::Value;

pub fn normalize_compact_header(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        _ => json!({ "value": value }),
    }
}
