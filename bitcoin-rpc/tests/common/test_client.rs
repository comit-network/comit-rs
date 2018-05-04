use bitcoin_rpc::*;
use self::super::client_factory::create_client;

pub struct BitcoinCoreTestClient {
    client: BitcoinCoreClient,
}

impl BitcoinCoreTestClient {
    pub fn new() -> BitcoinCoreTestClient {
        BitcoinCoreTestClient {
            client: create_client(),
        }
    }

    pub fn a_transaction_id(&self) -> TransactionId {
        let mut block = self.a_block();

        block.tx.remove(0)
    }

    pub fn a_block_hash(&self) -> BlockHash {
        self.a_block().hash
    }

    pub fn an_address(&self) -> Address {
        self.client
            .get_new_address()
            .unwrap()
            .into_result()
            .unwrap()
    }

    pub fn a_block(&self) -> Block {
        self.client
            .generate(1)
            .and_then(|response| {
                let blocks = response.into_result().unwrap();
                let block = blocks.get(0).unwrap();
                self.client.get_block(block)
            })
            .unwrap()
            .into_result()
            .unwrap()
    }
}
