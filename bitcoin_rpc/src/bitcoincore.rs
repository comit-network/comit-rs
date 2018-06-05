use jsonrpc::HTTPClient;
use jsonrpc::HTTPError;
use jsonrpc::header::{Authorization, Basic, Headers};
use jsonrpc::{JsonRpcVersion, RpcClient, RpcRequest, RpcResponse};
use serde::de::DeserializeOwned;
use types::Address;
use types::*;

pub struct BitcoinCoreClient {
    client: RpcClient,
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

        BitcoinCoreClient { client: rpc_client }
    }

    // Order as per: https://bitcoin.org/en/developer-reference#rpcs

    pub fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<&Address>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "addmultisigaddress",
            number_of_required_signatures,
            participants,
        ))
    }

    // TODO: abandontransaction
    // TODO: addmultisigaddress
    // TODO: addnode
    // TODO: addwitnessaddress
    // TODO: backupwallet
    // TODO: bumpfee
    // TODO: clearbanned
    // TODO: createmultisig

    pub fn create_raw_transaction(
        &self,
        inputs: Vec<&NewTransactionInput>,
        output: &NewTransactionOutput,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "createrawtransaction",
            inputs,
            output,
        ))
    }

    pub fn decode_rawtransaction(
        &self,
        tx: SerializedRawTransaction,
    ) -> Result<RpcResponse<DecodedRawTransaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "decoderawtransaction",
            tx,
        ))
    }

    pub fn decode_script(
        &self,
        script: RedeemScript,
    ) -> Result<RpcResponse<DecodedScript>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "decodescript",
            script,
        ))
    }

    // TODO: disconnectnode

    pub fn dump_privkey(&self, address: &Address) -> Result<RpcResponse<PrivateKey>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "dumpprivkey",
            address,
        ))
    }

    // TODO: dumpwallet
    // TODO: encryptwallet
    // TODO: estimatefee
    // TODO: estimatepriority

    pub fn fund_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        options: &FundingOptions,
    ) -> Result<RpcResponse<FundingResult>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "fundrawtransaction",
            tx,
            options,
        ))
    }

    pub fn generate(
        &self,
        number_of_blocks: u32,
    ) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "generate",
            number_of_blocks,
        ))
    }

    // TODO: generatetoaddress
    // TODO: getaccountaddress

    pub fn get_account(&self, address: &Address) -> Result<RpcResponse<Account>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getaccount",
            address,
        ))
    }

    // TODO: getaddednodeinfo
    // TODO: getaddressesbyaccount
    // TODO: getbalance
    // TODO: getbestblockhash

    pub fn get_block(&self, header_hash: &BlockHash) -> Result<RpcResponse<Block>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getblock",
            header_hash,
        ))
    }

    // TODO: getblockchaininfo

    pub fn get_block_count(&self) -> Result<RpcResponse<i32>, HTTPError> {
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getblockcount",
        ))
    }

    // TODO: getblockhash
    // TODO: getblockheader
    // TODO: getblocktemplate
    // TODO: getchaintips
    // TODO: getconnectioncount
    // TODO: getdifficulty
    // TODO: getgenerate
    // TODO: gethashespersec
    // TODO: getinfo
    // TODO: getmemoryinfo
    // TODO: getmempoolancestors
    // TODO: getmempooldescendants
    // TODO: getmempoolentry
    // TODO: getmempoolinfo
    // TODO: getmininginfo
    // TODO: getnettotals
    // TODO: getnetworkhashesps
    // TODO: getnetworkinfo

    pub fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "getnewaddress",
            "",
            "bech32",
        ))
    }

    // TODO: getpeerinfo
    // TODO: getrawchangeaddress
    // TODO: getrawmempool

    pub fn get_raw_transaction_serialized(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        self.get_raw_transaction(tx, false)
    }

    pub fn get_raw_transaction_verbose(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<VerboseRawTransaction>, HTTPError> {
        self.get_raw_transaction(tx, true)
    }

    fn get_raw_transaction<R>(
        &self,
        tx: &TransactionId,
        verbose: bool,
    ) -> Result<RpcResponse<R>, HTTPError>
    where
        R: DeserializeOwned,
    {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "getrawtransaction",
            tx,
            verbose,
        ))
    }

    // TODO: getreceivedbyaccount
    // TODO: getreceivedbyaddress

    pub fn get_transaction(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "gettransaction",
            tx,
        ))
    }

    // TODO: gettxout
    // TODO: gettxoutsetinfo
    // TODO: getunconfirmedbalance
    // TODO: getwalletinfo
    // TODO: getwork
    // TODO: importaddress
    // TODO: importmulti
    // TODO: importprivkey
    // TODO: importprunedfunds
    // TODO: importwallet
    // TODO: keypoolrefill
    // TODO: invalidateblock
    // TODO: keypoolrefill
    // TODO: listaccounts
    // TODO: listaddressgroupings
    // TODO: listbanned
    // TODO: listlockunspent
    // TODO: listreceivedbyaccount
    // TODO: listreceivedbyaddress
    // TODO: listsinceblock
    // TODO: listtransactions

    pub fn list_unspent(
        &self,
        min_confirmations: TxOutConfirmations,
        max_confirmations: Option<u32>,
        recipients: Option<Vec<Address>>,
    ) -> Result<RpcResponse<Vec<UnspentTransactionOutput>>, HTTPError> {
        let min_confirmations = match min_confirmations {
            TxOutConfirmations::Unconfirmed => 0,
            TxOutConfirmations::AtLeast(number) => number,
        };

        self.client.send(RpcRequest::new3(
            JsonRpcVersion::V1,
            "test",
            "listunspent",
            min_confirmations,
            max_confirmations,
            recipients,
        ))
    }

    // TODO: lockunspent
    // TODO: move
    // TODO: ping
    // TODO: preciousblock
    // TODO: prioritisetransaction
    // TODO: pruneblockchain
    // TODO: removeprunedfunds
    // TODO: sendfrom
    // TODO: sendmany

    pub fn send_raw_transaction(
        &self,
        tx_data: SerializedRawTransaction,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "sendrawtransaction",
            tx_data,
        ))
    }

    pub fn send_to_address(
        &self,
        address: Address,
        amount: f64,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "sendtoaddress",
            address,
            amount,
        ))
    }
    // TODO: setaccount
    // TODO: setban
    // TODO: setgenerate
    // TODO: setnetworkactive
    // TODO: settxfee
    // TODO: signmessage
    // TODO: signmessagewithprivkey

    pub fn sign_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        dependencies: Option<Vec<&TransactionOutputDetail>>,
        private_keys: Option<Vec<&PrivateKey>>,
        signature_hash_type: Option<SigHashType>,
    ) -> Result<RpcResponse<SigningResult>, HTTPError> {
        self.client.send(RpcRequest::new4(
            JsonRpcVersion::V1,
            "test",
            "signrawtransaction",
            tx,
            dependencies,
            private_keys,
            signature_hash_type,
        ))
    }

    // TODO: stop
    // TODO: submitblock

    pub fn validate_address(
        &self,
        address: &Address,
    ) -> Result<RpcResponse<AddressValidationResult>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "validateaddress",
            address,
        ))
    }

    // TODO: verifychain
    // TODO: verifymessage
    // TODO: verifytxoutproof
    // TODO: walletlock
    // TODO: walletpassphrase
    // TODO: walletpassphrasechange
}
