use bitcoin_rpc_client::{
    Address, BitcoinRpcApi, ClientError, RpcError, SerializedRawTransaction,
    TransactionId as BitcoinTxId,
};
use comit_node::swap_protocols::rfc003::ledger_htlc_service::BlockingEthereumApi;
use ethereum_support::{
    web3, Bytes, Transaction as EthereumTransaction, TransactionId as EthereumTxId,
    TransactionReceipt, H256,
};

pub struct BitcoinRpcClientMock {
    transaction_id: BitcoinTxId,
}

impl BitcoinRpcClientMock {
    pub fn new(transaction_id: BitcoinTxId) -> Self {
        BitcoinRpcClientMock { transaction_id }
    }
}

#[allow(unused_variables)]
impl BitcoinRpcApi for BitcoinRpcClientMock {
    fn send_raw_transaction(
        &self,
        _raw_transaction: SerializedRawTransaction,
    ) -> Result<Result<BitcoinTxId, RpcError>, ClientError> {
        Ok(Ok(self.transaction_id.clone()))
    }
    fn send_to_address(
        &self,
        _address: &Address,
        _amount: f64,
    ) -> Result<Result<BitcoinTxId, RpcError>, ClientError> {
        Ok(Ok(self.transaction_id.clone()))
    }
}

#[allow(dead_code)]
pub struct StaticEthereumApi;

impl BlockingEthereumApi for StaticEthereumApi {
    fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
        Ok(H256::new())
    }

    fn transaction(
        &self,
        _transaction_id: EthereumTxId,
    ) -> Result<Option<EthereumTransaction>, web3::Error> {
        unimplemented!()
    }

    fn transaction_receipt(
        &self,
        _transaction_id: H256,
    ) -> Result<Option<TransactionReceipt>, web3::Error> {
        unimplemented!()
    }
}
