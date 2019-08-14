#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

mod api;
#[macro_use]
pub mod json;

pub use crate::api::*;
