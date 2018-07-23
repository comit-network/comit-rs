use bitcoin_rpc::BitcoinCoreClient;
use coblox_bitcoincore::BitcoinCore;
use jsonrpc::{HTTPError, RpcResponse};
use std::fmt::Debug;
use testcontainers::{clients::DockerCli, Node};

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let node = Node::<BitcoinCoreClient, DockerCli>::new::<BitcoinCore>();

    let client = node.get_client();

    let result = invocation(client).unwrap().into_result();

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
