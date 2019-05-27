mod calculate_offsets;

use self::calculate_offsets::ethereum::rfc003::Error;
use crate::calculate_offsets::ethereum::rfc003::{contract::Contract, offset::to_markdown};
use std::ffi::OsStr;

const ETHER_TEMPLATE_FOLDER: &str = "./src/bin/calculate_offsets/ethereum/rfc003/templates/ether/";
const ERC20_TEMPLATE_FOLDER: &str = "./src/bin/calculate_offsets/ethereum/rfc003/templates/erc20/";

#[allow(clippy::print_stdout)]
fn main() -> Result<(), Error> {
    println!("### RFC003 ###");

    println!("{}", generate_markdown(ETHER_TEMPLATE_FOLDER)?);
    println!("{}", generate_markdown(ERC20_TEMPLATE_FOLDER)?);

    Ok(())
}

fn generate_markdown<S: AsRef<OsStr>>(template_folder: S) -> Result<String, Error> {
    let contract = contract(template_folder)?;

    let offsets = contract.placeholder_offsets()?;
    let metadata = contract.meta_data();

    Ok(format!(
        "{}\n{}",
        metadata.to_markdown(),
        to_markdown(offsets)
    ))
}

fn contract<S: AsRef<OsStr>>(template_folder: S) -> Result<Contract, Error> {
    Ok(Contract::compile(template_folder)?)
}

#[cfg(test)]
mod tests {
    use crate::{
        calculate_offsets::ethereum::rfc003::Error, contract, ERC20_TEMPLATE_FOLDER,
        ETHER_TEMPLATE_FOLDER,
    };
    use blockchain_contracts::ethereum::rfc003::{erc20_htlc, ether_htlc};

    #[test]
    fn ether_contract_template_matches_template_in_calculate_offsets() -> Result<(), Error> {
        let contract = contract(ETHER_TEMPLATE_FOLDER)?;
        assert_eq!(
            ether_htlc::CONTRACT_TEMPLATE.to_vec(),
            contract.meta_data().contract,
        );
        Ok(())
    }

    #[test]
    fn erc20_contract_template_matches_template_in_calculate_offsets() -> Result<(), Error> {
        let contract = contract(ERC20_TEMPLATE_FOLDER)?;
        assert_eq!(
            erc20_htlc::CONTRACT_TEMPLATE.to_vec(),
            contract.meta_data().contract,
        );
        Ok(())
    }
}
