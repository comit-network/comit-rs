extern crate env_logger;
extern crate ethereum_support;
extern crate ganache_rust_web3;
extern crate hex;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
#[macro_use]
extern crate log;
extern crate common_types;
extern crate tc_parity_parity;
extern crate testcontainers;
#[macro_use]
extern crate lazy_static;

mod parity_client;

#[test]
fn give_someone_ether() {
    //    let client = ParityClient::new();
    //    let address: Address = "147ba99ef89c152f8004e91999fee87bda6cbc3e".into();
    //    client.give_eth_to(address, EthereumQuantity::from_eth(1.0));
    //
    //    let balance = client.get_balance(address);
    //
    //    assert_eq!(balance, EthereumQuantity::from_eth(1.0).wei());
}
