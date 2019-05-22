extern crate regex;
use crate::calculate_offsets::ethereum::rfc003::{
    compile_contract::compile, metadata::Metadata, offset::Offset,
    placeholder_config::PlaceholderConfig, Error,
};
use std::{ffi::OsStr, path::PathBuf};

pub struct Contract {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
}

impl Contract {
    pub fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Contract, Error> {
        let mut bytes = compile(concat_path(&template_folder, "deploy_header.asm"))?;
        let mut contract_body = compile(concat_path(&template_folder, "contract.asm"))?;
        bytes.append(&mut contract_body);

        let placeholder_config =
            PlaceholderConfig::from_file(concat_path(&template_folder, "config.json"))?;

        Ok(Self {
            bytes,
            placeholder_config,
        })
    }

    pub fn placeholder_offsets(&self) -> Result<Vec<Offset>, Error> {
        self.placeholder_config
            .placeholders
            .iter()
            .map(|placeholder| {
                let decoded_placeholder = hex::decode(&placeholder.replace_pattern)?;
                let start_pos = Self::find_subsequence(&self.bytes[..], &decoded_placeholder[..])
                    .ok_or(Error::PlaceholderNotFound)?;
                let end_pos = start_pos + decoded_placeholder.len();
                Ok(Offset::new(
                    placeholder.name.to_owned(),
                    start_pos,
                    end_pos,
                    decoded_placeholder.len(),
                ))
            })
            .collect()
    }

    pub fn meta_data(&self) -> Metadata {
        Metadata {
            ledger_name: self.placeholder_config.ledger_name.to_owned(),
            asset_name: self.placeholder_config.asset_name.to_owned(),
            contract_hex: hex::encode(self.bytes.to_owned()),
        }
    }

    fn find_subsequence(contract_template: &[u8], placeholder: &[u8]) -> Option<usize> {
        contract_template
            .windows(placeholder.len())
            .position(|window| window == placeholder)
    }
}

fn concat_path<S: AsRef<OsStr>>(folder: S, file: &str) -> PathBuf {
    [OsStr::new(&folder), OsStr::new(file)].iter().collect()
}
