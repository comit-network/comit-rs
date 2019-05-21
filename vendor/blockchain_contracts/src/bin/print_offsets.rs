mod calculate_offsets;

use self::calculate_offsets::ethereum::rfc003::{offset::to_markdown, Error};
use crate::calculate_offsets::ethereum::rfc003::contract::Contract;
use std::ffi::OsStr;

#[allow(clippy::print_stdout)]
fn main() -> Result<(), Error> {
    println!("### RFC003 ###");

    print_offsets("./src/bin/calculate_offsets/ethereum/rfc003/templates/ether/")?;
    print_offsets("./src/bin/calculate_offsets/ethereum/rfc003/templates/erc20/")?;

    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_offsets<S: AsRef<OsStr>>(template_folder: S) -> Result<(), Error> {
    let contract = Contract::compile_from_directory_and_load_placeholder_config(template_folder)?;

    let offsets = contract.calculate_offsets()?;
    let metadata = contract.meta_data;

    println!(
        "** {} on {} **",
        &metadata.asset_name, &metadata.ledger_name
    );
    println!("Contract template:\n {}", metadata.contract_hex);
    println!("{}", to_markdown(offsets));

    Ok(())
}
