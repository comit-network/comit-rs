use ethereum_htlc;
use ethereum_support::*;
use ganache_rust_web3;
use hex;
use tc_trufflesuite_ganachecli::GanacheCli;
use tc_web3_client::Web3Client;
use testcontainers::{clients::DockerCli, Container, Docker};

const TOKEN_CONTRACT_CODE: &'static str = include_str!("standard_erc20_token_contract.asm.hex");

pub struct GanacheClient {
    _container: Container<DockerCli, GanacheCli>,
    client: Web3Client,
    snapshot_id: Option<ganache_rust_web3::SnapshotId>,
}

impl GanacheClient {
    pub fn new() -> Self {
        let container = DockerCli::new().run(GanacheCli::default());
        let client = Web3Client::new(&container);

        GanacheClient {
            _container: container,
            client,
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

    pub fn deploy_token_contract(&self, from: Address) -> Address {
        let contract_tx_id = self
            .client
            .eth()
            .send_transaction(TransactionRequest {
                from,
                to: None,
                gas: Some(U256::from(4_000_000u64)),
                gas_price: None,
                value: None,
                data: Some(Bytes(hex::decode(TOKEN_CONTRACT_CODE.trim()).unwrap())),
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

    pub fn mint_1000_tokens(
        &self,
        contract_owner: Address,
        contract: Address,
        to: Address,
    ) -> U256 {
        let function_identifier = "40c10f19";
        let address = format!("000000000000000000000000{}", hex::encode(to));
        let amount = format!("00000000000000000000000000000000000000000000000000000000000003e8");

        let payload = format!("{}{}{}", function_identifier, address, amount);

        self.send_data(
            contract_owner,
            contract,
            Some(Bytes(hex::decode(payload).unwrap())),
        )
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

    pub fn get_token_balance(&self, contract: Address, address: Address) -> U256 {
        let function_identifier = "70a08231";
        let address_hex = format!("000000000000000000000000{}", hex::encode(address));

        let payload = format!("{}{}", function_identifier, address_hex);

        let result = self
            .client
            .eth()
            .call(
                CallRequest {
                    from: Some(address),
                    to: contract,
                    gas: None,
                    gas_price: None,
                    value: None,
                    data: Some(Bytes(hex::decode(payload).unwrap())),
                },
                None,
            )
            .wait()
            .unwrap();

        U256::from(result.0.as_slice())
    }
}
