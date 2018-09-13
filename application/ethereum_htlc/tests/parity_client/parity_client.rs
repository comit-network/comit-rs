use ethereum_htlc;
use ethereum_support::{Address, Bytes, EthereumQuantity, Future, TransactionRequest, U256};
use tc_parity_parity::ParityEthereum;
use tc_web3_client::Web3Client;
use testcontainers::{clients::DockerCli, Container, Docker};

pub struct ParityClient {
    _container: Container<DockerCli, ParityEthereum>,
    client: Web3Client,
}

lazy_static! {
    static ref PARITY_DEV_ACCOUNT: Address =
        "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();
}

const PARITY_DEV_PASSWORD: &str = "";

impl ParityClient {
    pub fn new() -> Self {
        let container = DockerCli::new().run(ParityEthereum::default());
        let client = Web3Client::new(&container);

        ParityClient {
            _container: container,
            client,
        }
    }

    pub fn give_eth_to(&self, to: Address, amount: EthereumQuantity) {
        self.client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: PARITY_DEV_ACCOUNT.clone(),
                    to: Some(to),
                    gas: None,
                    gas_price: None,
                    value: Some(amount.wei()),
                    data: None,
                    nonce: None,
                    condition: None,
                },
                PARITY_DEV_PASSWORD,
            )
            .wait()
            .unwrap();
    }

    pub fn deploy(&self, from: Address, htlc: ethereum_htlc::Htlc, htlc_value: i32) -> Address {
        let compiled_contract = htlc.compile_to_hex();

        let contract_tx_id = self
            .client
            .eth()
            .send_transaction(TransactionRequest {
                from: from,
                to: None,
                gas: None,
                gas_price: None,
                value: Some(U256::from(htlc_value)),
                data: Some(compiled_contract.into()),
                nonce: None,
                condition: None,
            })
            .wait()
            .unwrap();

        let receipt = self
            .client
            .eth()
            .transaction_receipt(contract_tx_id)
            .wait()
            .unwrap()
            .unwrap();

        debug!("Deploying the contract consumed {} gas", receipt.gas_used);

        receipt.contract_address.unwrap()
    }

    pub fn send_data(&self, from: Address, to: Address, data: Option<Bytes>) -> U256 {
        let result_tx = self
            .client
            .eth()
            .send_transaction(TransactionRequest {
                from: from,
                to: Some(to),
                gas: None,
                gas_price: None,
                value: None,
                data: data,
                nonce: None,
                condition: None,
            })
            .wait()
            .unwrap();

        let receipt = self
            .client
            .eth()
            .transaction_receipt(result_tx)
            .wait()
            .unwrap()
            .unwrap();

        receipt.gas_used
    }

    pub fn get_balance(&self, address: Address) -> U256 {
        self.client.eth().balance(address, None).wait().unwrap()
    }
}
