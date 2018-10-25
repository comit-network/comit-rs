use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use rocket;
use routes::{
    bitcoin::{self, block_query::BitcoinBlockQuery, transaction_query::BitcoinTransactionQuery},
    ethereum::{
        self, block_query::EthereumBlockQuery, transaction_query::EthereumTransactionQuery,
    },
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
        transaction_query_repository: Arc<QueryRepository<BitcoinTransactionQuery>>,
        transaction_query_result_repository: Arc<QueryResultRepository<BitcoinTransactionQuery>>,
        block_query_repository: Arc<QueryRepository<BitcoinBlockQuery>>,
        block_query_result_repository: Arc<QueryResultRepository<BitcoinBlockQuery>>,
    ) -> ServerBuilder {
        let rocket = self
            .rocket
            .mount(
                "/",
                routes![
                    bitcoin::transaction_query::handle_new_query,
                    bitcoin::transaction_query::retrieve_query,
                    bitcoin::transaction_query::delete_query,
                    bitcoin::block_query::handle_new_query,
                    bitcoin::block_query::retrieve_query,
                    bitcoin::block_query::delete_query,
                ],
            ).manage(transaction_query_repository)
            .manage(transaction_query_result_repository)
            .manage(block_query_repository)
            .manage(block_query_result_repository);
        ServerBuilder { rocket }
    }

    pub fn register_ethereum(
        self,
        transaction_query_repository: Arc<QueryRepository<EthereumTransactionQuery>>,
        transaction_query_result_repository: Arc<QueryResultRepository<EthereumTransactionQuery>>,
        block_query_repository: Arc<QueryRepository<EthereumBlockQuery>>,
        block_query_result_repository: Arc<QueryResultRepository<EthereumBlockQuery>>,
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
            ).manage(transaction_query_repository)
            .manage(transaction_query_result_repository)
            .manage(block_query_repository)
            .manage(block_query_result_repository);
        ServerBuilder { rocket }
    }
}
