use bitcoin_rpc_api::BitcoinRpcApi;
use bitcoincore::*;
use jsonrpc::HTTPError;
use jsonrpc::RpcResponse;
use std::fmt::Debug;
use testcontainers::clients::DockerCli;
use testcontainers::images::{Bitcoind, BitcoindImageArgs};
use testcontainers::{Container, Docker, Image, RunArgs};
use types::*;

pub struct BitcoinNode {
    container_id: String,
    docker: DockerCli,
    client: BitcoinCoreClient,
}

impl BitcoinNode {
    pub fn new() -> Self {
        let docker = DockerCli {};

        let args = BitcoindImageArgs {
            rpc_auth: "bitcoin:cb77f0957de88ff388cf817ddbc7273$9eaa166ace0d94a29c6eceb831a42458e93faeb79f895a7ee4ce03f4343f8f55".to_string(),
            ..BitcoindImageArgs::default()
        };

        let bitcoind = Bitcoind::new("0.16.0").with_args(args);

        let container_id = docker.run_detached(
            &bitcoind,
            RunArgs {
                ports: bitcoind.exposed_ports(),
                rm: true,
                ..RunArgs::default()
            },
        );
        let info = docker.inspect(&container_id);

        let external_port = info.ports().map_to_external_port(18443).unwrap();

        let url = format!("http://localhost:{}", external_port);

        let username = "bitcoin";
        let password = "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=";

        let client = BitcoinCoreClient::new(url.as_str(), username, password);

        BitcoinNode {
            container_id,
            docker,
            client,
        }
    }

    pub fn get_client(&self) -> &BitcoinCoreClient {
        &self.client
    }
}

impl Drop for BitcoinNode {
    fn drop(&mut self) {
        self.docker.rm(&self.container_id);
    }
}

pub fn assert_successful_result<R, I>(invocation: I)
where
    R: Debug,
    I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
{
    let node = BitcoinNode::new();

    let client = node.get_client();

    let result = invocation(client).unwrap().into_result();

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
