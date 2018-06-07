use bitcoincore::TxOutConfirmations;
use jsonrpc::HTTPError;
use jsonrpc::JsonRpcVersion;
use jsonrpc::RpcRequest;
use jsonrpc::RpcResponse;
use serde::de::DeserializeOwned;
use types::*;

pub trait BitcoinRpcApi: Send + Sync {
    // Order as per: https://bitcoin.org/en/developer-reference#rpcs

    fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<&Address>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError>;

    // TODO: abandontransaction
    // TODO: addmultisigaddress
    // TODO: addnode
    // TODO: addwitnessaddress
    // TODO: backupwallet
    // TODO: bumpfee
    // TODO: clearbanned
    // TODO: createmultisig

    fn create_raw_transaction(
        &self,
        inputs: Vec<&NewTransactionInput>,
        output: &NewTransactionOutput,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError>;

    fn decode_rawtransaction(
        &self,
        tx: SerializedRawTransaction,
    ) -> Result<RpcResponse<DecodedRawTransaction>, HTTPError>;

    fn decode_script(&self, script: RedeemScript) -> Result<RpcResponse<DecodedScript>, HTTPError>;

    // TODO: disconnectnode

    fn dump_privkey(&self, address: &Address) -> Result<RpcResponse<PrivateKey>, HTTPError>;

    // TODO: dumpwallet
    // TODO: encryptwallet
    // TODO: estimatefee
    // TODO: estimatepriority

    fn fund_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        options: &FundingOptions,
    ) -> Result<RpcResponse<FundingResult>, HTTPError>;

    fn generate(&self, number_of_blocks: u32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError>;

    // TODO: generatetoaddress
    // TODO: getaccountaddress

    fn get_account(&self, address: &Address) -> Result<RpcResponse<Account>, HTTPError>;

    // TODO: getaddednodeinfo
    // TODO: getaddressesbyaccount
    // TODO: getbalance
    // TODO: getbestblockhash

    fn get_block(&self, header_hash: &BlockHash) -> Result<RpcResponse<Block>, HTTPError>;

    // TODO: getblockchaininfo

    fn get_block_count(&self) -> Result<RpcResponse<BlockHeight>, HTTPError>;

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

    fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError>;

    // TODO: getpeerinfo
    // TODO: getrawchangeaddress
    // TODO: getrawmempool

    fn get_raw_transaction_serialized(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError>;

    fn get_raw_transaction_verbose(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<VerboseRawTransaction>, HTTPError>;

    // TODO: getreceivedbyaccount
    // TODO: getreceivedbyaddress

    fn get_transaction(&self, tx: &TransactionId) -> Result<RpcResponse<Transaction>, HTTPError>;

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

    fn list_unspent(
        &self,
        min_confirmations: TxOutConfirmations,
        max_confirmations: Option<u32>,
        recipients: Option<Vec<Address>>,
    ) -> Result<RpcResponse<Vec<UnspentTransactionOutput>>, HTTPError>;

    // TODO: lockunspent
    // TODO: move
    // TODO: ping
    // TODO: preciousblock
    // TODO: prioritisetransaction
    // TODO: pruneblockchain
    // TODO: removeprunedfunds
    // TODO: sendfrom
    // TODO: sendmany

    fn send_raw_transaction(
        &self,
        tx_data: SerializedRawTransaction,
    ) -> Result<RpcResponse<TransactionId>, HTTPError>;

    fn send_to_address(
        &self,
        address: &Address,
        amount: f64,
    ) -> Result<RpcResponse<TransactionId>, HTTPError>;
    // TODO: setaccount
    // TODO: setban
    // TODO: setgenerate
    // TODO: setnetworkactive
    // TODO: settxfee
    // TODO: signmessage
    // TODO: signmessagewithprivkey

    fn sign_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        dependencies: Option<Vec<&TransactionOutputDetail>>,
        private_keys: Option<Vec<&PrivateKey>>,
        signature_hash_type: Option<SigHashType>,
    ) -> Result<RpcResponse<SigningResult>, HTTPError>;

    // TODO: stop
    // TODO: submitblock

    fn validate_address(
        &self,
        address: &Address,
    ) -> Result<RpcResponse<AddressValidationResult>, HTTPError>;

    // TODO: verifychain
    // TODO: verifymessage
    // TODO: verifytxoutproof
    // TODO: walletlock
    // TODO: walletpassphrase
    // TODO: walletpassphrasechange
}
