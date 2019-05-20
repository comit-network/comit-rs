mod calculate_offsets;

use self::calculate_offsets::ethereum::rfc003::{
    calculate_offsets::calculate_offsets, offset::to_markdown, Error,
};

#[allow(clippy::print_stdout)]
fn main() {
    println!("### RFC003 ###");

    print_offsets("ether");
    print_offsets("erc20");
}

fn print_offsets(asset: &'static str) -> Result<(), Error> {
    let offsets = calculate_offsets(asset)?;

    println!("** {} on {} **", offsets.asset_name, offsets.ledger_name);
    println!("Contract template:\n {}", offsets.contract);
    println!("{}", to_markdown(offsets.offsets));

    Ok(())
}
