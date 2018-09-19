use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use rocket;
use routes::{
    bitcoin_query::{self, BitcoinQuery},
    ethereum_query::{self, EthereumQuery},
};
use std::sync::Arc;

pub fn create(
    config: rocket::Config,
    link_factory: LinkFactory,
    bitcoin_repositories: Option<(
        Arc<QueryRepository<BitcoinQuery>>,
        Arc<QueryResultRepository<BitcoinQuery>>,
    )>,
    ethereum_repositories: Option<(
        Arc<QueryRepository<EthereumQuery>>,
        Arc<QueryResultRepository<EthereumQuery>>,
    )>,
) -> rocket::Rocket {
    let mut rocket = rocket::custom(config, true);

    if let Some((query_repository, query_result_repository)) = bitcoin_repositories {
        rocket = rocket
            .mount(
                "/",
                routes![
                    bitcoin_query::handle_new_bitcoin_query,
                    bitcoin_query::retrieve_bitcoin_query,
                    bitcoin_query::delete_bitcoin_query,
                ],
            )
            .manage(query_repository)
            .manage(query_result_repository);
    }

    if let Some((query_repository, query_result_repository)) = ethereum_repositories {
        rocket = rocket
            .mount(
                "/",
                routes![
                    ethereum_query::handle_new_ethereum_query,
                    ethereum_query::retrieve_ethereum_query,
                    ethereum_query::delete_ethereum_query,
                ],
            )
            .manage(query_repository)
            .manage(query_result_repository);
    }

    rocket.manage(link_factory)
}
