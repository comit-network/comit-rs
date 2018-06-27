use bitcoin_rpc::*;
use jsonrpc::HTTPError;
use jsonrpc::RpcError;
use jsonrpc::RpcResponse;
use std::fmt::Debug;
use testcontainers;
use testcontainers::{Container, Docker, Image, RunArgs};

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let docker = testcontainers::clients::DockerCli {};
    let bitcoind = testcontainers::images::Bitcoind::latest();

    let id = docker.run_detached(
        &bitcoind,
        RunArgs {
            ports: bitcoind.exposed_ports(),
            rm: true,
            ..RunArgs::default()
        },
    );
    let info = docker.inspect(&id);

    let external_port = info.ports().map_to_external_port(18443).unwrap();

    let url = format!("http://localhost:{}", external_port);

    let username = "bitcoin";
    let password = "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=";

    let client = BitcoinCoreClient::new(url.as_str(), username, password);

    let result: Result<R, RpcError> = invocation(&client).unwrap().into();

    docker.rm(&id);

    if result.is_err() {
        error!("{:?}", result.unwrap_err());
        panic!("Result should be successful")
    } else {
        // Having a successful result means:
        // - No HTTP Error occured
        // - No deserialization error occured
        debug!("{:?}", result.unwrap())
    }
}
