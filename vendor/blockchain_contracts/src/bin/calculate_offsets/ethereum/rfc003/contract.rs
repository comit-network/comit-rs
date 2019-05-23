extern crate regex;
use crate::calculate_offsets::ethereum::rfc003::{
    compile_contract::compile,
    metadata::Metadata,
    offset::Offset,
    placeholder_config::{Placeholder, PlaceholderConfig},
    Error,
};
use byteorder::{BigEndian, ByteOrder};
use std::{ffi::OsStr, path::PathBuf};

pub struct Contract {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
}

impl Contract {
    pub fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Contract, Error> {
        let mut bytes = compile(concat_path(&template_folder, "deploy_header.asm"))?;
        let mut contract_body = compile(concat_path(&template_folder, "contract.asm"))?;

        Self::replace_contract_offset_parameters_in_header(&mut bytes, &contract_body)?;

        bytes.append(&mut contract_body);

        let placeholder_config =
            PlaceholderConfig::from_file(concat_path(&template_folder, "config.json"))?;

        Ok(Self {
            bytes,
            placeholder_config,
        })
    }

    fn replace_contract_offset_parameters_in_header(
        header: &mut [u8],
        body: &[u8],
    ) -> Result<(), Error> {
        let body_length = body.len();
        let header_length = header.len();

        Self::replace_offset_parameter_in_header(
            "1001",
            "start of contract when loading into memory",
            header_length,
            header,
        )?;
        Self::replace_offset_parameter_in_header(
            "2002",
            "end of contract when loading into memory",
            body_length,
            header,
        )?;
        Self::replace_offset_parameter_in_header(
            "3003",
            "length of contract when returning for execution",
            body_length,
            header,
        )?;

        Ok(())
    }

    fn replace_offset_parameter_in_header(
        replace_pattern: &str,
        name: &str,
        value: usize,
        header: &mut [u8],
    ) -> Result<(), Error> {
        let header_placeholder = Placeholder {
            name: name.into(),
            replace_pattern: replace_pattern.into(),
        };

        let header_placeholder_offset = Self::calc_offset(&header_placeholder, header)?;

        let header_slice =
            &mut header[header_placeholder_offset.start..header_placeholder_offset.excluded_end];

        BigEndian::write_u16(header_slice, value as u16);

        Ok(())
    }

    pub fn placeholder_offsets(&self) -> Result<Vec<Offset>, Error> {
        self.placeholder_config
            .placeholders
            .iter()
            .map(|placeholder| Self::calc_offset(placeholder, &self.bytes))
            .collect()
    }

    fn calc_offset(placeholder: &Placeholder, contract: &[u8]) -> Result<Offset, Error> {
        let decoded_placeholder = hex::decode(placeholder.replace_pattern.as_str())?;
        let start_pos = Self::find_subsequence(&contract[..], &decoded_placeholder[..])
            .ok_or(Error::PlaceholderNotFound)?;
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

    pub fn meta_data(&self) -> Metadata {
        Metadata {
            ledger_name: self.placeholder_config.ledger_name.to_owned(),
            asset_name: self.placeholder_config.asset_name.to_owned(),
            contract: self.bytes.to_owned(),
        }
    }
}

fn concat_path<S: AsRef<OsStr>>(folder: S, file: &str) -> PathBuf {
    [OsStr::new(&folder), OsStr::new(file)].iter().collect()
}
