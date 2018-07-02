use ethereum_htlc;
use ganache_node::GanacheCliNode;
use ganache_rust_web3;
use web3;
use web3::futures::Future;
use web3::types::Address;
use web3::types::Bytes;
use web3::types::TransactionRequest;
use web3::types::U256;

pub struct GanacheClient {
    ganache_node: GanacheCliNode,
    snapshot_id: Option<ganache_rust_web3::SnapshotId>,
}

impl GanacheClient {
    pub fn new() -> Self {
        let ganache_node = GanacheCliNode::new();

        GanacheClient {
            ganache_node,
            snapshot_id: None,
        }
    }

    pub fn take_snapshot(&mut self) {
        self.snapshot_id = Some(
            self.ganache_node
                .get_client()
                .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
                .evm_snapshot()
                .wait()
                .unwrap(),
        );
    }

    pub fn restore_snapshot(&self) {
        self.ganache_node
            .get_client()
            .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
            .evm_revert(self.snapshot_id.as_ref().unwrap())
            .wait()
            .unwrap();
    }

    pub fn deploy(&self, from: Address, htlc: ethereum_htlc::Htlc, htlc_value: i32) -> Address {
        let compiled_contract = htlc.compile_to_hex();

        let contract_tx_id = self.ganache_node
            .get_client()
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

        let receipt = self.ganache_node
            .get_client()
            .eth()
            .transaction_receipt(contract_tx_id)
            .wait()
            .unwrap()
            .unwrap();

        debug!("Deploying the contract consumed {} gas", receipt.gas_used);

        receipt.contract_address.unwrap()
    }

    pub fn send_data(&self, from: Address, to: Address, data: Option<Bytes>) -> U256 {
        let result_tx = self.ganache_node
            .get_client()
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

        let receipt = self.ganache_node
            .get_client()
            .eth()
            .transaction_receipt(result_tx)
            .wait()
            .unwrap()
            .unwrap();

        receipt.gas_used
    }

    pub fn activate_flux_compensator(&self, hours: u64) {
        let _ = self.ganache_node
            .get_client()
            .api::<ganache_rust_web3::Ganache<web3::transports::Http>>()
            .evm_increase_time(60 * 60 * hours)
            .wait()
            .unwrap();
    }

    pub fn get_balance(&self, address: Address) -> U256 {
        self.ganache_node
            .get_client()
            .eth()
            .balance(address, None)
            .wait()
            .unwrap()
    }
}
