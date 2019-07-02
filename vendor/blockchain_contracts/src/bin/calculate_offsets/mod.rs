use crate::calculate_offsets::{
    metadata::Metadata,
    offset::Offset,
    placeholder_config::{Placeholder, PlaceholderConfig},
};
use std::{ffi::OsStr, path::PathBuf, process::Command};

pub mod bitcoin;
pub mod ethereum;
pub mod metadata;
pub mod offset;
pub mod placeholder_config;

#[derive(Debug)]
pub enum Error {
    PlaceholderNotFound(String),
    Hex(hex::FromHexError),
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::Hex(e)
    }
}

pub trait Contract: std::marker::Sized {
    type Error: From<Error>;

    fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Self, Self::Error>;
    fn metadata(&self) -> Metadata;
    fn placeholder_config(&self) -> &PlaceholderConfig;
    fn bytes(&self) -> &Vec<u8>;
}

pub fn placeholder_offsets<C: Contract>(contract: C) -> Result<Vec<Offset>, Error> {
    contract
        .placeholder_config()
        .placeholders
        .iter()
        .map(|placeholder| calc_offset(placeholder, contract.bytes()))
        .collect()
}

fn concat_path<S: AsRef<OsStr>>(folder: S, file: &str) -> PathBuf {
    [OsStr::new(&folder), OsStr::new(file)].iter().collect()
}

fn calc_offset(placeholder: &Placeholder, contract: &[u8]) -> Result<Offset, Error> {
    let decoded_placeholder = hex::decode(placeholder.replace_pattern.as_str())?;
    let start_pos = find_subsequence(&contract[..], &decoded_placeholder[..])
        .ok_or_else(|| Error::PlaceholderNotFound(hex::encode(&decoded_placeholder)))?;
    let end_pos = start_pos + decoded_placeholder.len();
    Ok(Offset::new(
        placeholder.name.to_owned(),
        start_pos,
        end_pos,
        decoded_placeholder.len(),
    ))
}

fn find_subsequence(contract_template: &[u8], placeholder: &[u8]) -> Option<usize> {
    contract_template
        .windows(placeholder.len())
        .position(|window| window == placeholder)
}

fn check_bin_in_path(bin: &str) {
    let output = Command::new("which").arg(bin).output().unwrap();
    if output.stdout.is_empty() {
        let msg = format!(
            "`{}` cannot be found, check your path\nPATH: {:?}",
            bin,
            std::env::var("PATH")
        );
        panic!(msg);
    }
}
