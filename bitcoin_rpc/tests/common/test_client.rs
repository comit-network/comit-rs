use bitcoin_rpc::*;

pub struct BitcoinCoreTestClient<'a> {
    pub client: &'a BitcoinCoreClient,
}

impl<'a> BitcoinCoreTestClient<'a> {
    pub fn new(client: &'a BitcoinCoreClient) -> BitcoinCoreTestClient {
        BitcoinCoreTestClient { client }
    }

    pub fn a_utxo(&self) -> UnspentTransactionOutput {
        let _ = self.a_block(); // Need to generate a block first

        let mut utxos = self.client
            .list_unspent(TxOutConfirmations::AtLeast(6), None, None)
            .unwrap()
            .into_result()
            .unwrap();

        utxos.remove(0)
    }

    pub fn a_transaction_id(&self) -> TransactionId {
        let mut block = self.a_block();

        block.tx.remove(0)
    }

    pub fn a_block_hash(&self) -> BlockHash {
        self.a_block().hash
    }

    pub fn an_address(&self) -> RpcAddress {
        self.client
            .get_new_address()
            .unwrap()
            .into_result()
            .unwrap()
    }

    pub fn a_block(&self) -> Block {
        self.client
            .generate(101)
            .and_then(|response| {
                let blocks = response.into_result().unwrap();
                let block = blocks.get(50).unwrap();
                self.client.get_block(block)
            })
            .unwrap()
            .into_result()
            .unwrap()
    }
}
