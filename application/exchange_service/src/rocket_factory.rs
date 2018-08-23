use bitcoin_fee_service::BitcoinFeeService;
use bitcoin_rpc_client;
use bitcoin_support::{self, Network};
use ethereum_service::EthereumService;
use ethereum_support;
use event_store::InMemoryEventStore;
use rocket;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swaps::{common::TradeId, eth_btc};
use treasury_api_client::ApiClient;

pub fn create_rocket_instance(
    treasury_api_client: Arc<ApiClient>,
    event_store: InMemoryEventStore<TradeId>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_rpc_client: Arc<bitcoin_rpc_client::BitcoinRpcApi>,
    exchange_refund_address: ethereum_support::Address,
    exchange_success_keypair: KeyPair,
    btc_exchange_redeem_address: bitcoin_support::Address,
    network: Network,
    bitcoin_fee_service: Arc<BitcoinFeeService>,
) -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes![
                eth_btc::buy::routes::post_buy_offers,
                eth_btc::buy::routes::post_buy_orders,
                eth_btc::buy::routes::post_orders_funding,
                eth_btc::buy::routes::post_revealed_secret,
                eth_btc::sell::routes::post_orders_funding,
                eth_btc::sell::routes::post_revealed_secret,
            ],
        )
        .manage(treasury_api_client)
        .manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_rpc_client)
        .manage(exchange_success_keypair)
        .manage(exchange_refund_address)
        .manage(btc_exchange_redeem_address)
        .manage(network)
        .manage(bitcoin_fee_service)
}
