use BitcoinRpcApi;
use jsonrpc::HTTPClient;
use jsonrpc::HTTPError;
use jsonrpc::header::{Authorization, Basic, Headers};
use jsonrpc::{JsonRpcVersion, RpcClient, RpcRequest, RpcResponse};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::fmt::Debug;
use types::Address;
use types::*;

struct RetryConfig {
    max_retries: u32,
    interval: u64,
}

pub struct BitcoinCoreClient {
    client: RpcClient,
    retry_config: Option<RetryConfig>,
}

pub enum TxOutConfirmations {
    Unconfirmed,
    AtLeast(i32),
}

#[allow(dead_code)]
impl BitcoinCoreClient {
    pub fn new(url: &str, username: &str, password: &str) -> Self {
        let mut headers = Headers::new();
        headers.set(Authorization(Basic {
            username: username.to_string(),
            password: Some(password.to_string()),
        }));

        let client = HTTPClient::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let rpc_client = RpcClient::new(client, url);

        BitcoinCoreClient {
            client: rpc_client,
            retry_config: Some(RetryConfig {
                max_retries: 10,
                interval: 500,
            }),
        }
    }

    fn get_raw_transaction<R: Debug>(
        &self,
        tx: &TransactionId,
        verbose: bool,
    ) -> Result<RpcResponse<R>, HTTPError>
    where
        R: DeserializeOwned,
    {
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "getrawtransaction",
            tx,
            verbose,
        ))
    }
}

impl BitcoinRpcApi for BitcoinCoreClient {
    // Order as per: https://bitcoin.org/en/developer-reference#rpcs

    fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<&Address>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError> {
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "addmultisigaddress",
            number_of_required_signatures,
            participants,
        ))
    }

    fn create_raw_transaction(
        &self,
        inputs: Vec<&NewTransactionInput>,
        output: &NewTransactionOutput,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        //TODO: Should we add a check for inputs to cleanly exist?
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "createrawtransaction",
            inputs,
            output,
        ))
    }

    fn decode_rawtransaction(
        &self,
        tx: SerializedRawTransaction,
    ) -> Result<RpcResponse<DecodedRawTransaction>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "decoderawtransaction",
            tx,
        ))
    }

    fn decode_script(&self, script: RedeemScript) -> Result<RpcResponse<DecodedScript>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "decodescript",
            script,
        ))
    }

    fn dump_privkey(&self, address: &Address) -> Result<RpcResponse<PrivateKey>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "dumpprivkey",
            address,
        ))
    }

    fn fund_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        options: &FundingOptions,
    ) -> Result<RpcResponse<FundingResult>, HTTPError> {
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "fundrawtransaction",
            tx,
            options,
        ))
    }

    fn generate(&self, number_of_blocks: u32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "generate",
            number_of_blocks,
        ))
    }

    fn get_account(&self, address: &Address) -> Result<RpcResponse<Account>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getaccount",
            address,
        ))
    }

    fn get_block(&self, header_hash: &BlockHash) -> Result<RpcResponse<Block>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getblock",
            header_hash,
        ))
    }

    fn get_blockchain_info(&self) -> Result<RpcResponse<BlockchainInfo>, HTTPError> {
        self.send(&RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getblockchaininfo",
        ))
    }

    fn get_block_count(&self) -> Result<RpcResponse<BlockHeight>, HTTPError> {
        self.send(&RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getblockcount",
        ))
    }

    fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError> {
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "getnewaddress",
            "",
            "bech32",
        ))
    }

    fn get_raw_transaction_serialized(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        self.get_raw_transaction(tx, false)
    }

    fn get_raw_transaction_verbose(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<VerboseRawTransaction>, HTTPError> {
        self.get_raw_transaction(tx, true)
    }

    fn get_transaction(&self, tx: &TransactionId) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "gettransaction",
            tx,
        ))
    }

    fn list_unspent(
        &self,
        min_confirmations: TxOutConfirmations,
        max_confirmations: Option<u32>,
        recipients: Option<Vec<Address>>,
    ) -> Result<RpcResponse<Vec<UnspentTransactionOutput>>, HTTPError> {
        let min_confirmations = match min_confirmations {
            TxOutConfirmations::Unconfirmed => 0,
            TxOutConfirmations::AtLeast(number) => number,
        };

        self.send(&RpcRequest::new3(
            JsonRpcVersion::V1,
            "test",
            "listunspent",
            min_confirmations,
            max_confirmations,
            recipients,
        ))
    }

    fn send_raw_transaction(
        &self,
        tx_data: SerializedRawTransaction,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "sendrawtransaction",
            tx_data,
        ))
    }

    fn send_to_address(
        &self,
        address: &Address,
        amount: f64,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        self.send(&RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "sendtoaddress",
            address,
            amount,
        ))
    }

    fn sign_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        dependencies: Option<Vec<&TransactionOutputDetail>>,
        private_keys: Option<Vec<&PrivateKey>>,
        signature_hash_type: Option<SigHashType>,
    ) -> Result<RpcResponse<SigningResult>, HTTPError> {
        self.send(&RpcRequest::new4(
            JsonRpcVersion::V1,
            "test",
            "signrawtransaction",
            tx,
            dependencies,
            private_keys,
            signature_hash_type,
        ))
    }

    fn validate_address(
        &self,
        address: &Address,
    ) -> Result<RpcResponse<AddressValidationResult>, HTTPError> {
        self.send(&RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "validateaddress",
            address,
        ))
    }
}

impl BitcoinCoreClient {
    fn send<R: DeserializeOwned + Debug, P: Serialize + Debug>(
        &self,
        request: &RpcRequest<P>,
    ) -> Result<RpcResponse<R>, HTTPError> {
        if let Some(ref config) = self.retry_config {
            for i in 0..config.max_retries {
                let result = self.client.send::<R, P>(request);

                match result {
                    Ok(rpc_response) => match rpc_response.error {
                        Some(ref rpc_error) if rpc_error.code == -28 => {
                            info!("Bitcoind is still starting up. Request will be retried in {} milliseconds. ({}/{}) ", config.interval, i, config.max_retries);

                            ::std::thread::sleep(::std::time::Duration::from_millis(
                                config.interval,
                            ));
                            continue;
                        }
                        _ => return Ok(rpc_response),
                    },
                    Err(http_error) => return Err(http_error),
                }
            }
        }
        self.client.send(request)
    }
}
