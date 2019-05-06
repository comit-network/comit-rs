#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

use std::ops::Range;

pub mod ethereum;
pub mod rfc003;

#[derive(Debug)]
pub struct OffsetParameter {
    pub value: Vec<u8>,
    pub range: Range<usize>,
}
