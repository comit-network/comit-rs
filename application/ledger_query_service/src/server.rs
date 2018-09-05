use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use rocket;
use routes::bitcoin_query::{self, BitcoinQuery};
use std::sync::Arc;

pub fn create(
    config: rocket::Config,
    link_factory: LinkFactory,
    bitcoin_query_repository: Arc<QueryRepository<BitcoinQuery>>,
    bitcoin_query_result_repository: Arc<QueryResultRepository>,
) -> rocket::Rocket {
    rocket::custom(config, true)
        .mount(
            "/",
            routes![
                bitcoin_query::handle_new_bitcoin_query,
                bitcoin_query::retrieve_bitcoin_query,
                bitcoin_query::delete_bitcoin_query,
            ],
        )
        .manage(link_factory)
        .manage(bitcoin_query_repository)
        .manage(bitcoin_query_result_repository)
}
