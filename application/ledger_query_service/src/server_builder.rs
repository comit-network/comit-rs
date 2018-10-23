use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use rocket;
use routes::{
    bitcoin::{self, transaction_query::BitcoinTransactionQuery},
    ethereum::{self, transaction_query::EthereumTransactionQuery},
};
use std::sync::Arc;

#[derive(DebugStub)]
pub struct ServerBuilder {
    #[debug_stub = "Rocket"]
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
        query_repository: Arc<QueryRepository<BitcoinTransactionQuery>>,
        query_result_repository: Arc<QueryResultRepository<BitcoinTransactionQuery>>,
    ) -> ServerBuilder {
        let rocket = self
            .rocket
            .mount(
                "/",
                routes![
                    bitcoin::transaction_query::handle_new_query,
                    bitcoin::transaction_query::retrieve_query,
                    bitcoin::transaction_query::delete_query,
                ],
            ).manage(query_repository)
            .manage(query_result_repository);
        ServerBuilder { rocket }
    }

    pub fn register_ethereum(
        self,
        query_repository: Arc<QueryRepository<EthereumTransactionQuery>>,
        query_result_repository: Arc<QueryResultRepository<EthereumTransactionQuery>>,
    ) -> ServerBuilder {
        let rocket = self
            .rocket
            .mount(
                "/",
                routes![
                    ethereum::transaction_query::handle_new_query,
                    ethereum::transaction_query::retrieve_query,
                    ethereum::transaction_query::delete_query,
                ],
            ).manage(query_repository)
            .manage(query_result_repository);
        ServerBuilder { rocket }
    }
}
