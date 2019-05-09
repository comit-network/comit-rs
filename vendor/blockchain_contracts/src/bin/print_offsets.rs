use self::calculate_offsets::{
    ethereum::rfc003::{erc20_offsets, ether_offsets},
    to_markdown,
};

mod calculate_offsets;

#[allow(clippy::print_stdout)]
fn main() {
    println!("### RFC003 ###");

    {
        println!("** Ether on Ethereum **");
        let contract = ether_offsets::compile_template_to_hex();
        println!("Contract template:\n {}", contract);
        let offsets = ether_offsets::get_all_offsets();
        println!("{}", to_markdown(offsets));
    }

    {
        println!("** ERC20 on Ethereum **");
        let contract = erc20_offsets::compile_template_to_hex();
        println!("Contract template:\n {}", contract);
        let offsets = erc20_offsets::all_offsets();
        println!("{}", to_markdown(offsets));
    }
}
