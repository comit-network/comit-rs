use crate::calculate_offsets::ethereum::rfc003::Error;
use serde::Deserialize;
use std::{ffi::OsStr, fs::File, io::BufReader};

#[derive(Debug, Deserialize)]
pub struct PlaceholderConfig {
    pub ledger_name: String,
    pub asset_name: String,
    pub placeholders: Vec<Placeholder>,
}

#[derive(Debug, Deserialize)]
pub struct Placeholder {
    pub name: String,
    pub replace_pattern: String,
}

impl PlaceholderConfig {
    pub fn from_file<S: AsRef<OsStr>>(file_path: S) -> Result<PlaceholderConfig, Error> {
        let file = File::open(OsStr::new(&file_path))?;
        let reader = BufReader::new(file);

        let config = serde_json::from_reader(reader)?;

        Ok(config)
    }
}
