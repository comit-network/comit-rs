use bitcoin_rpc;
use ethereum_service::EthereumService;
use event_store::EventStore;
use rocket;
use routes;
use std::sync::Arc;
use treasury_api_client::ApiClient;

pub fn create_rocket_instance(
    treasury_api_client: Arc<ApiClient>,
    event_store: EventStore,
    ethereum_service: Arc<EthereumService>,
    bitcoin_rpc_client: Arc<bitcoin_rpc::BitcoinRpcApi>,
) -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes![
                routes::eth_btc::post_buy_offers,
                routes::eth_btc::post_buy_orders,
                routes::eth_btc::post_buy_orders_fundings
            ],
        )
        .manage(treasury_api_client)
        .manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_rpc_client)
}
