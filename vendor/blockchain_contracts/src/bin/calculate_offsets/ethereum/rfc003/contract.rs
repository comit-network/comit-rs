extern crate regex;
use crate::calculate_offsets::ethereum::rfc003::{
    compile_contract::compile, metadata::Metadata, offset::Offset,
    placeholder_config::PlaceholderConfig, Error,
};
use std::{ffi::OsStr, path::PathBuf};

pub struct Contract {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
    pub meta_data: Metadata,
}

impl Contract {
    pub fn compile_from_directory_and_load_placeholder_config<S: AsRef<OsStr>>(
        template_folder: S,
    ) -> Result<Contract, Error> {
        let mut bytes = compile(concat_path(&template_folder, "deploy_header.asm"))?;
        let mut contract_body = compile(concat_path(&template_folder, "contract.asm"))?;
        bytes.append(&mut contract_body);

        let placeholder_config =
            PlaceholderConfig::from_file(concat_path(&template_folder, "config.json"))?;

        let meta_data = Metadata::new(
            placeholder_config.ledger_name.to_owned(),
            placeholder_config.ledger_name.to_owned(),
            hex::encode(bytes.to_owned()),
        );

        Ok(Self {
            bytes,
            placeholder_config,
            meta_data,
        })
    }

    pub fn calculate_offsets(&self) -> Result<Vec<Offset>, Error> {
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

    fn find_subsequence(contract_template: &[u8], placeholder: &[u8]) -> Option<usize> {
        contract_template
            .windows(placeholder.len())
            .position(|window| window == placeholder)
    }
}

fn concat_path<S: AsRef<OsStr>>(folder: S, file: &str) -> PathBuf {
    [OsStr::new(&folder), OsStr::new(file)].iter().collect()
}
