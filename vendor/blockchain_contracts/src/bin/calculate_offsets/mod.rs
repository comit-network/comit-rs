use serde::Serialize;
use std::cmp::Ordering;

pub mod ethereum;

#[derive(Clone, Copy, strum_macros::Display, Debug, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum DataName {
    SecretHash,
    Expiry,
    RedeemIdentity,
    RefundIdentity,
    TokenQuantity,
    TokenContract,
}

#[derive(Debug)]
pub struct Offset {
    start: usize,
    excluded_end: usize,
    length: usize,
    data: DataName,
}

pub fn to_markdown(mut offsets: Vec<Offset>) -> String {
    let mut res = String::from("| Data | Byte Range | Length (bytes) |\n|:--- |:--- |:--- |");
    offsets.sort_unstable();
    for offset in offsets {
        res = format!("{}\n{}", res, offset.row_format())
    }
    res
}

impl Offset {
    fn new(data: DataName, start: usize, excluded_end: usize, length: usize) -> Offset {
        Offset {
            data,
            start,
            excluded_end,
            length,
        }
    }

    fn row_format(&self) -> String {
        format!(
            "| `{}` | {}..{} | {} |",
            self.data, self.start, self.excluded_end, self.length
        )
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

impl Eq for Offset {}
