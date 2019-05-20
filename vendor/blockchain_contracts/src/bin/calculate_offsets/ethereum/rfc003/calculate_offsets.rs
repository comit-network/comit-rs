use crate::calculate_offsets::ethereum::rfc003::{
    compile_contract::compile, contract_config::ContractConfig, offset::Offset, offsets::Offsets,
    Error,
};
use std::{fs::File, io::BufReader};

const TEMPLATE_FOLDER: &str = "./src/bin/calculate_offsets/ethereum/rfc003/templates/";
const CONFIG_FILE: &str = "config.json";
const CONTRACT_FILE: &str = "contract.asm";
const HEADER_FILE: &str = "deploy_header.asm";

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn calculate_offsets(asset: &'static str) -> Result<Offsets, Error> {
    let config = load_config(asset)?;
    let contract = compile_contract(asset)?;

    let mut offsets: Vec<Offset> = Vec::new();

    for placeholder in config.placeholders {
        let decoded_placeholder = hex::decode(placeholder.replace_pattern)?;
        let start_pos = find_subsequence(&contract[..], &decoded_placeholder[..])
            .ok_or(Error::PlaceholderNotFound)?;
        let end_pos = start_pos + decoded_placeholder.len();
        let offset = Offset::new(
            placeholder.name,
            start_pos,
            end_pos,
            decoded_placeholder.len(),
        );

        offsets.push(offset);
    }

    Ok(Offsets::new(
        config.ledger_name,
        config.asset_name,
        hex::encode(contract),
        offsets,
    ))
}

fn compile_contract(asset: &'static str) -> Result<Vec<u8>, Error> {
    let mut contract_header = compile(&get_header_file_path())?;
    let mut contract_body = compile(&get_contract_file_path(asset))?;

    contract_header.append(&mut contract_body);

    Ok(contract_header)
}

fn load_config(asset: &'static str) -> Result<ContractConfig, Error> {
    let file = File::open(get_config_file_path(asset))?;
    let reader = BufReader::new(file);

    let config = serde_json::from_reader(reader)?;

    Ok(config)
}

fn get_config_file_path(asset: &'static str) -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + asset + "/" + CONFIG_FILE;
    file_path
}

fn get_contract_file_path(asset: &'static str) -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + asset + "/" + CONTRACT_FILE;
    file_path
}

fn get_header_file_path() -> String {
    let file_path = TEMPLATE_FOLDER.to_owned() + HEADER_FILE;
    file_path
}
