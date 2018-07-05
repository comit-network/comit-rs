use BitcoinRpcApi;
use NewTransactionOutput;
use bitcoincore::TxOutConfirmations;
use jsonrpc::HTTPError;
use jsonrpc::RpcResponse;
use types::*;

pub struct BitcoinStubClient {}

impl BitcoinStubClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(unused_variables)]
impl BitcoinRpcApi for BitcoinStubClient {
    fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<&RpcAddress>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError> {
        unimplemented!()
    }

    fn create_raw_transaction(
        &self,
        inputs: Vec<&NewTransactionInput>,
        output: &NewTransactionOutput,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        unimplemented!()
    }

    fn decode_rawtransaction(
        &self,
        tx: SerializedRawTransaction,
    ) -> Result<RpcResponse<DecodedRawTransaction>, HTTPError> {
        unimplemented!()
    }

    fn decode_script(&self, script: RedeemScript) -> Result<RpcResponse<DecodedScript>, HTTPError> {
        unimplemented!()
    }

    fn dump_privkey(&self, address: &RpcAddress) -> Result<RpcResponse<RpcPrivateKey>, HTTPError> {
        unimplemented!()
    }

    fn fund_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        options: &FundingOptions,
    ) -> Result<RpcResponse<FundingResult>, HTTPError> {
        unimplemented!()
    }

    fn generate(&self, number_of_blocks: u32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        unimplemented!()
    }

    fn get_account(&self, address: &RpcAddress) -> Result<RpcResponse<Account>, HTTPError> {
        unimplemented!()
    }

    fn get_block(&self, header_hash: &BlockHash) -> Result<RpcResponse<Block>, HTTPError> {
        unimplemented!()
    }

    fn get_blockchain_info(&self) -> Result<RpcResponse<Blockchain>, HTTPError> {
        unimplemented!()
    }

    fn get_block_count(&self) -> Result<RpcResponse<BlockHeight>, HTTPError> {
        unimplemented!()
    }

    fn get_new_address(&self) -> Result<RpcResponse<RpcAddress>, HTTPError> {
        unimplemented!()
    }

    fn get_raw_transaction_serialized(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<SerializedRawTransaction>, HTTPError> {
        unimplemented!()
    }

    fn get_raw_transaction_verbose(
        &self,
        tx: &TransactionId,
    ) -> Result<RpcResponse<VerboseRawTransaction>, HTTPError> {
        unimplemented!()
    }

    fn get_transaction(&self, tx: &TransactionId) -> Result<RpcResponse<Transaction>, HTTPError> {
        unimplemented!()
    }

    fn list_unspent(
        &self,
        min_confirmations: TxOutConfirmations,
        max_confirmations: Option<u32>,
        recipients: Option<Vec<RpcAddress>>,
    ) -> Result<RpcResponse<Vec<UnspentTransactionOutput>>, HTTPError> {
        unimplemented!()
    }

    fn send_raw_transaction(
        &self,
        tx_data: SerializedRawTransaction,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        unimplemented!()
    }

    fn send_to_address(
        &self,
        address: &RpcAddress,
        amount: f64,
    ) -> Result<RpcResponse<TransactionId>, HTTPError> {
        unimplemented!()
    }

    fn sign_raw_transaction(
        &self,
        tx: &SerializedRawTransaction,
        dependencies: Option<Vec<&TransactionOutputDetail>>,
        private_keys: Option<Vec<&RpcPrivateKey>>,
        signature_hash_type: Option<SigHashType>,
    ) -> Result<RpcResponse<SigningResult>, HTTPError> {
        unimplemented!()
    }

    fn validate_address(
        &self,
        address: &RpcAddress,
    ) -> Result<RpcResponse<AddressValidationResult>, HTTPError> {
        unimplemented!()
    }
}
