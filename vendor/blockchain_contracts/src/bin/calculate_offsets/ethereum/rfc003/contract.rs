extern crate regex;
use crate::calculate_offsets::ethereum::rfc003::{
    compile_contract::compile, metadata::Metadata, offset::Offset,
    placeholder_config::PlaceholderConfig, Error,
};

const TEMPLATE_FOLDER: &str = "./src/bin/calculate_offsets/ethereum/rfc003/templates/";
const CONTRACT_FILE: &str = "contract.asm";
const HEADER_FILE: &str = "deploy_header.asm";
const CONFIG_FILE: &str = "config.json";

pub struct Contract {
    bytes: Vec<u8>,
    placeholder_config: PlaceholderConfig,
    pub meta_data: Metadata,
}

impl Contract {
    pub fn compile_from_directory_and_load_placeholder_config(
        dir: &'static str,
    ) -> Result<Contract, Error> {
        let mut bytes = compile(&get_header_file_path())?;
        let mut contract_body = compile(&get_contract_file_path(dir))?;
        bytes.append(&mut contract_body);

        let placeholder_config = PlaceholderConfig::from_file(&get_config_file_path(dir))?;

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
        let mut offsets: Vec<Offset> = Vec::new();

        for placeholder in &self.placeholder_config.placeholders {
            let decoded_placeholder = hex::decode(&placeholder.replace_pattern)?;
            let start_pos = Self::find_subsequence(&self.bytes[..], &decoded_placeholder[..])
                .ok_or(Error::PlaceholderNotFound)?;
            let end_pos = start_pos + decoded_placeholder.len();
            let offset = Offset::new(
                placeholder.name.to_owned(),
                start_pos,
                end_pos,
                decoded_placeholder.len(),
            );

            offsets.push(offset);
        }

        Ok(offsets)
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }
}

fn get_contract_file_path(asset: &'static str) -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + asset + "/" + CONTRACT_FILE;
    file_path
}

fn get_header_file_path() -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + HEADER_FILE;
    file_path
}

fn get_config_file_path(asset: &'static str) -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + asset + "/" + CONFIG_FILE;
    file_path
}
