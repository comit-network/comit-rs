use comit_node::swap_protocols::rfc003::ethereum::Htlc;
use ethereum_support::{
    web3::{
        transports::{EventLoopHandle, Http},
        Web3,
    },
    *,
};
use ganache_rust_web3;
use tc_trufflesuite_ganachecli::GanacheCli;
use tc_web3_client;
use testcontainers::{clients::DockerCli, Container, Docker};

pub struct GanacheClient {
    _container: Container<DockerCli, GanacheCli>,
    _event_loop: EventLoopHandle,
    client: Web3<Http>,
    snapshot_id: Option<ganache_rust_web3::SnapshotId>,
}

impl GanacheClient {
    pub fn new() -> Self {
        let container = DockerCli::new().run(GanacheCli::default());

        let (event_loop, web3) = tc_web3_client::new(&container);

        GanacheClient {
            _container: container,
            _event_loop: event_loop,
            client: web3,
            snapshot_id: None,
        }
    }

    pub fn take_snapshot(&mut self) {
        self.snapshot_id = Some(
            self.client
                .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
                .evm_snapshot()
                .wait()
                .unwrap(),
        );
    }

    pub fn restore_snapshot(&self) {
        self.client
            .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
            .evm_revert(self.snapshot_id.as_ref().unwrap())
            .wait()
            .unwrap();
    }

    pub fn deploy<H: Htlc>(&self, from: Address, htlc: H, htlc_value: i32) -> Address {
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
            .wait();

        let contract_tx_id = contract_tx_id.unwrap();

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

    pub fn activate_flux_capacitor(&self, hours: u64) {
        let _ = self
            .client
            .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
            .evm_increase_time(60 * 60 * hours)
            .wait()
            .unwrap();
    }

    pub fn get_balance(&self, address: Address) -> U256 {
        self.client.eth().balance(address, None).wait().unwrap()
    }
}
