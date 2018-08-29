use common::alice_and_bob::{Alice, Bob};
use pretty_env_logger;
use tokio::runtime::Runtime;

use common::alice_and_bob;
use transport_protocol::{config::Config, json::*};

pub fn setup(config: Config<Request, Response>) -> (Runtime, Alice, Bob) {
    let _ = pretty_env_logger::try_init();

    alice_and_bob::create(config)
}
