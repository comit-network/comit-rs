use common::alice_and_bob::{Alice, Bob};
use futures::Future;
use pretty_env_logger;
use tokio::runtime::Runtime;

use common::alice_and_bob;
use transport_protocol::{
    config::Config,
    connection,
    json::{self, *},
    shutdown_handle,
};

pub fn start_server_with(config: Config<Request, Response>) -> (Runtime, Alice, Bob) {
    pretty_env_logger::try_init();
    let mut runtime = Runtime::new().unwrap();

    let (alice, bob_server, alice_client) = alice_and_bob::create(config);
    let (bob_server, bob_shutdown_handle) = shutdown_handle::new(bob_server);
    runtime.spawn(bob_server.map_err(|_| ()));

    let bob = Bob {
        _alice: alice_client,
        _shutdown_handle: bob_shutdown_handle,
    };

    (runtime, alice, bob)
}

pub fn create_server_with(
    config: Config<Request, Response>,
) -> (
    Alice,
    impl Future<Item = (), Error = connection::ClosedReason<json::Error>>,
) {
    pretty_env_logger::try_init();
    let (alice, bob_server, _alice_client) = alice_and_bob::create(config);
    (alice, bob_server)
}
