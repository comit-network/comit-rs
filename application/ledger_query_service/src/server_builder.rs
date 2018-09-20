use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use rocket;
use routes::{
    bitcoin_query::{self, BitcoinQuery},
    ethereum_query::{self, EthereumQuery},
};
use std::sync::Arc;

pub struct ServerBuilder {
    rocket: rocket::Rocket,
}

impl ServerBuilder {
    pub fn create(config: rocket::Config, link_factory: LinkFactory) -> ServerBuilder {
        let rocket = rocket::custom(config, true).manage(link_factory);
        ServerBuilder { rocket }
    }

    pub fn build(self) -> rocket::Rocket {
        self.rocket
    }

    pub fn register_bitcoin(
        self,
        query_repository: Arc<QueryRepository<BitcoinQuery>>,
        query_result_repository: Arc<QueryResultRepository<BitcoinQuery>>,
    ) -> ServerBuilder {
        let rocket = self
            .rocket
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
        ServerBuilder { rocket }
    }

    pub fn register_ethereum(
        self,
        query_repository: Arc<QueryRepository<EthereumQuery>>,
        query_result_repository: Arc<QueryResultRepository<EthereumQuery>>,
    ) -> ServerBuilder {
        let rocket = self
            .rocket
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
        ServerBuilder { rocket }
    }
}
