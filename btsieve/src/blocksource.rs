use futures::Future;

pub trait BlockSource {
    type Error: std::fmt::Debug;
    type Block;
    type BlockHash;
    type Network;

    fn network(&self) -> Self::Network;

    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
}

pub trait TransactionReceiptBlockSource: BlockSource {
    type TransactionReceipt;
    type TransactionHash;

    fn transaction_receipt(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::TransactionReceipt, Error = Self::Error> + Send + 'static>;
}
