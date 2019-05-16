use self::calculate_offsets::{
    ethereum::rfc003::{erc20_offsets::Erc20Offsets, ether_offsets::EtherOffsets},
    to_markdown,
};

mod calculate_offsets;

#[allow(clippy::print_stdout)]
fn main() {
    println!("### RFC003 ###");

    {
        println!("** Ether on Ethereum **");
        let contract = EtherOffsets::new().contract_template();
        println!("Contract template:\n {}", contract);
        let offsets = EtherOffsets::new().get_all_offsets();
        println!("{}", to_markdown(offsets));
    }

    {
        println!("** ERC20 on Ethereum **");
        let contract = Erc20Offsets::new().contract_template();
        println!("Contract template:\n {}", contract);
        let offsets = Erc20Offsets::new().all_offsets();
        println!("{}", to_markdown(offsets));
    }
}
