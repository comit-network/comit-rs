use bitcoin_node::BitcoinNode;
use bitcoin_rpc::BitcoinCoreClient;
use jsonrpc::{HTTPError, RpcResponse};
use std::fmt::Debug;

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let node = BitcoinNode::new();

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
