use bitcoin_support::Network;
use comit_wallet::KeyStore;
use event_store::InMemoryEventStore;
use ledger_query_service::{BitcoinQuery, EthereumQuery};
use rocket::{
    self,
    config::{Config, Environment},
    Rocket,
};
use std::sync::Arc;
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::ledger_htlc_service::{
        BitcoinHtlcParams, BitcoinHtlcRedeemParams, EtherHtlcParams, EtherHtlcRedeemParams,
        LedgerHtlcService,
    },
};
use swaps::{common::TradeId, eth_btc};

pub fn create_rocket_instance(
    event_store: Arc<InMemoryEventStore<TradeId>>,
    ethereum_service: Arc<
        LedgerHtlcService<Ethereum, EtherHtlcParams, EtherHtlcRedeemParams, EthereumQuery>,
    >,
    bitcoin_service: Arc<
        LedgerHtlcService<Bitcoin, BitcoinHtlcParams, BitcoinHtlcRedeemParams, BitcoinQuery>,
    >,
    my_keystore: Arc<KeyStore>,
    network: Network,
    address: String,
    port: u16,
    logging: bool,
) -> rocket::Rocket {
    try_config(address, port, logging)
        .mount(
            "/ledger/", //Endpoints for notifying about ledger events
            routes![
                // TODO will be removed once we have the Ledger Query Service
                eth_btc::ledger::buy_routes::post_contract_deployed,
            ],
        ).manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_service)
        .manage(my_keystore)
        .manage(network)
}

fn try_config(address: String, port: u16, logging: bool) -> Rocket {
    //TODO change environment?
    let config = Config::build(Environment::Development)
        .address(address.clone())
        .port(port)
        .finalize();
    match config {
        Ok(config) => rocket::custom(config, logging),
        Err(error) => {
            error!("{:?}", error);
            error!(
                "Could not start rocket with {}:{}, falling back to default",
                address, port
            );
            rocket::ignite()
        }
    }
}
