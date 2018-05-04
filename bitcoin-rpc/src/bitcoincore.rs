use jsonrpc::HTTPClient;
use jsonrpc::{JsonRpcVersion, RpcClient, RpcRequest, RpcResponse};
use jsonrpc::header::{Authorization, Basic, Headers};
use types::Address;
use jsonrpc::HTTPError;
use types::*;

pub struct BitcoinCoreClient {
    client: RpcClient,
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

    // TODO: abandontransaction
    // TODO: addmultisigaddress
    // TODO: addnode
    // TODO: addwitnessaddress
    // TODO: backupwallet
    // TODO: bumpfee
    // TODO: clearbanned
    // TODO: createmultisig
    // TODO: createrawtransaction

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
    // TODO: dumpprivkey
    // TODO: dumpwallet
    // TODO: encryptwallet
    // TODO: estimatefee
    // TODO: estimatepriority
    // TODO: fundrawtransaction

    pub fn generate(
        &self,
        number_of_blocks: i32,
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

    pub fn get_account(&self, address: Address) -> Result<RpcResponse<Account>, HTTPError> {
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
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getnewaddress",
        ))
    }

    // TODO: getpeerinfo
    // TODO: getrawchangeaddress
    // TODO: getrawmempool

    pub fn get_raw_transaction_serialized(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "getrawtransaction",
            tx,
            false,
        ))
    }

    // TODO: getreceivedbyaccount
    // TODO: getreceivedbyaddress

    pub fn get_transaction(
        &self,
        tx: TransactionId,
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
    // TODO: listunspent
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
    ) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "sendrawtransaction",
            tx_data,
        ))
    }

    // TODO: sendtoaddress
    // TODO: setaccount
    // TODO: setban
    // TODO: setgenerate
    // TODO: setnetworkactive
    // TODO: settxfee
    // TODO: signmessage
    // TODO: signmessagewithprivkey
    // TODO: signrawtransaction
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
