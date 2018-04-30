use jsonrpc::HTTPClient;
use jsonrpc::header;
use jsonrpc::{JsonRpcVersion, RpcClient, RpcRequest, RpcResponse};
use jsonrpc::header::{Authorization, Basic, Headers};
use types::Address;
use jsonrpc::HTTPError;
use types::*;

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

    fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<Address>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "addmultisigaddress",
            number_of_required_signatures,
            participants,
        ))
    }

    fn get_block_count(&self) -> Result<RpcResponse<i32>, HTTPError> {
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getblockcount",
        ))
    }

    fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError> {
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getnewaddress",
        ))
    }

    fn generate(&self, number_of_blocks: i32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "generate",
            number_of_blocks,
        ))
    }

    fn get_account(&self, address: Address) -> Result<RpcResponse<Account>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getaccount",
            address,
        ))
    }

    fn get_transaction(&self, tx: TransactionId) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "gettransaction",
            tx,
        ))
    }

    fn send_raw_transaction(
        &self,
        tx_data: RawTransactionHex,
    ) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "sendrawtransaction",
            tx_data,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpc::RpcError;
    use std::fmt::Debug;

    fn assert_successful_result<R>(
        invocation: fn(client: &BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
    ) where
        R: Debug,
    {
        let client = BitcoinCoreClient::new();
        let result: Result<R, RpcError> = invocation(&client).unwrap().into();

        if result.is_err() {
            println!("{:?}", result.unwrap_err());
            panic!("Result should be successful")
        } else {
            // Having a successful result means:
            // - No HTTP Error occured
            // - No deserialization error occured
            println!("{:?}", result.unwrap())
        }
    }

    #[test]
    fn test_add_multisig_address() {
        assert_successful_result(|client| {
            let address = client.get_new_address().unwrap().to_result().unwrap();

            client.add_multisig_address(1, vec![address])
        })
    }

    #[test]
    fn test_get_block_count() {
        assert_successful_result(BitcoinCoreClient::get_block_count)
    }

    #[test]
    fn test_get_new_address() {
        assert_successful_result(BitcoinCoreClient::get_new_address)
    }

    #[test]
    fn test_generate() {
        assert_successful_result(|client| client.generate(1))
    }

    #[test]
    fn test_getaccount() {
        assert_successful_result(|client| {
            client.get_account(Address::from("2N2PMtfaKc9knQYxmTZRg3dcC1SMZ7SC8PW"))
        })
    }

    #[test]
    fn test_gettransaction() {
        assert_successful_result(|client| {
            client.get_transaction(TransactionId::from(
                "7e7c52b1f46e7ea2511e885d8c0e5df9297f65b6fff6907ceb1377d0582e45f4",
            ))
        })
    }
}
