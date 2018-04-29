use jsonrpc::HTTPClient;
use jsonrpc::header;
use jsonrpc::{RpcClient, JsonRpcVersion, RpcRequest, RpcResponse};
use jsonrpc::header::{Authorization, Basic, Headers};
use types::Address;
use jsonrpc::HTTPError;
use types::BlockHash;

struct BitcoinCoreClient {
    client: RpcClient,
}

impl BitcoinCoreClient {
    pub fn new() -> Self {
        let mut headers = Headers::new();
        headers.set(Authorization(Basic {
            username: "user".to_string(),
            password: Some("password".to_string()),
        }));

        let client = HTTPClient::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let rpc_client = RpcClient::new(client, "http://127.0.0.1:8332");

        BitcoinCoreClient { client: rpc_client }
    }

    fn get_block_count(&self) -> Result<RpcResponse<i32>, HTTPError> {
        self.client.send(RpcRequest::new0(JsonRpcVersion::V1, "test", "getblockcount"))
    }

    fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError> {
        self.client.send(RpcRequest::new0(JsonRpcVersion::V1, "test", "getnewaddress"))
    }

    fn generate(&self, number_of_blocks: i32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        self.client.send(RpcRequest::new1(JsonRpcVersion::V1, "test", "generate", number_of_blocks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_block_count() {
        let bitcoin_rpc_client = BitcoinCoreClient::new();

        let response = bitcoin_rpc_client.get_block_count().unwrap();

        println!("result: {:?}", response);
    }

    #[test]
    fn test_get_new_address() {
        let bitcoin_rpc_client = BitcoinCoreClient::new();

        let response = bitcoin_rpc_client.get_new_address().unwrap();

        println!("result: {:?}", response);
    }

    #[test]
    fn test_generate_block() {
        let bitcoin_rpc_client = BitcoinCoreClient::new();

        let response = bitcoin_rpc_client.generate(1).unwrap();

        println!("{:?}", response);
    }
}
