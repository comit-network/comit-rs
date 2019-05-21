mod calculate_offsets;

use self::calculate_offsets::ethereum::rfc003::Error;
use crate::calculate_offsets::ethereum::rfc003::{contract::Contract, offset::to_markdown};
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
    let contract = Contract::compile(template_folder)?;

    let offsets = contract.placeholder_offsets()?;
    let metadata = contract.meta_data;

    println!("{}", metadata.to_markdown());
    println!("{}", to_markdown(offsets));

    Ok(())
}
