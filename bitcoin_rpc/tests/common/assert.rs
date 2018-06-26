use self::super::client_factory::create_client;
use bitcoin_rpc::*;
use jsonrpc::HTTPError;
use jsonrpc::RpcError;
use jsonrpc::RpcResponse;
use std::fmt::Debug;

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let client = create_client();
    let result: Result<R, RpcError> = invocation(&client).unwrap().into();

    let _ = client.stop();

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
