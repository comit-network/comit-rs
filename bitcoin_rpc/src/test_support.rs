use bitcoin_rpc_api::BitcoinRpcApi;
use bitcoincore::*;
use jsonrpc::HTTPError;
use jsonrpc::RpcError;
use jsonrpc::RpcResponse;
use std::fmt::Debug;
use testcontainers;
use testcontainers::{Container, Docker, Image, RunArgs};
use types::*;

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let docker = testcontainers::clients::DockerCli {};
    let bitcoind = testcontainers::images::Bitcoind::latest();

    let id = docker.run_detached(
        &bitcoind,
        RunArgs {
            ports: bitcoind.exposed_ports(),
            rm: true,
            ..RunArgs::default()
        },
    );
    let info = docker.inspect(&id);

    let external_port = info.ports().map_to_external_port(18443).unwrap();

    let url = format!("http://localhost:{}", external_port);

    let username = "bitcoin";
    let password = "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=";

    let client = BitcoinCoreClient::new(url.as_str(), username, password);

    let result: Result<R, RpcError> = invocation(&client).unwrap().into();

    docker.rm(&id);

    if result.is_err() {
        error!("{:?}", result.unwrap_err());
        panic!("Result should be successful")
    } else {
        // Having a successful result means:
        // - No HTTP Error occured
        // - No deserialization error occured
        debug!("{:?}", result.unwrap())
    }
}

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

    pub fn an_address(&self) -> Address {
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
