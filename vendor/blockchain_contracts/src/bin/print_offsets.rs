use blockchain_contracts::{
    ethereum::rfc003::{Erc20Htlc, EtherHtlc},
    format_table,
};

#[allow(clippy::print_stdout)]
fn main() {
    println!("### RFC003 ###");

    {
        println!("** Ether on Ethereum **");
        let contract = EtherHtlc::compile_template_to_hex();
        println!("Contract template:\n {}", contract);
        let offsets = EtherHtlc::get_all_offsets();
        println!("{}", format_table(offsets));
    }

    {
        println!("** ERC20 on Ethereum **");
        let contract = Erc20Htlc::compile_template_to_hex();
        println!("Contract template:\n {}", contract);
        let offsets = Erc20Htlc::all_offsets();
        println!("{}", format_table(offsets));
    }
}
