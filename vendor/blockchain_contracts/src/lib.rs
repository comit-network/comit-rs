#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate serde;

use std::{cmp::Ordering, fmt::Display};

pub mod ethereum;
pub mod rfc003;

#[derive(Debug, Eq)]
pub struct Offset {
    data: String,
    start: usize,
    end: usize,
    length: usize,
}

impl Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}\t{}\t\t{}\t{}",
            self.data, self.start, self.end, self.length
        )
    }
}

pub fn format_table(mut offsets: Vec<Offset>) -> String {
    let mut res = String::from("Data\t\t\t\tStart\tEnd\tLength");
    offsets.sort_unstable();
    for offset in offsets {
        res = format!("{}\n{}", res, offset)
    }
    res
}

impl Offset {
    fn new(data: String, start: usize, end: usize, length: usize) -> Offset {
        Offset {
            data,
            start,
            end,
            length,
        }
    }
}

impl PartialEq for Offset {
    fn eq(&self, other: &Offset) -> bool {
        self.start == other.start
    }
}

impl PartialOrd for Offset {
    fn partial_cmp(&self, other: &Offset) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Offset {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}
