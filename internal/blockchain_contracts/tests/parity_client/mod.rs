use crate::ethereum_helper::{UnsignedTransaction, Wallet};
use lazy_static::lazy_static;
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};
use web3::{
    futures::Future,
    transports::Http,
    types::{Address, Bytes, CallRequest, TransactionReceipt, TransactionRequest, H256, U256},
    Web3,
};

#[allow(missing_debug_implementations)]
pub struct ParityClient {
    client: Arc<Web3<Http>>,
    wallet: Arc<dyn Wallet>,
    nonce: Mutex<U256>,
}

lazy_static! {
    static ref PARITY_DEV_ACCOUNT: Address =
        "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();
}

const ERC20_TOKEN_CONTRACT_CODE: &str = include_str!("erc20_token_contract.asm.hex");

const PARITY_DEV_PASSWORD: &str = "";

impl ParityClient {
    pub fn new<N: Into<U256>>(
        wallet: Arc<dyn Wallet>,
        client: Arc<Web3<Http>>,
        current_nonce: N,
    ) -> Self {
        ParityClient {
            wallet,
            nonce: Mutex::new(current_nonce.into()),
            client,
        }
    }

    pub fn give_eth_to(&self, to: Address, wei_amount: U256) {
        self.client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: *PARITY_DEV_ACCOUNT,
                    to: Some(to),
                    gas: None,
                    gas_price: None,
                    value: Some(wei_amount),
                    data: None,
                    nonce: None,
                    condition: None,
                },
                PARITY_DEV_PASSWORD,
            )
            .wait()
            .unwrap();
    }

    pub fn deploy_erc20_token_contract(&self) -> Address {
        let contract_tx_id = self
            .client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: *PARITY_DEV_ACCOUNT,
                    to: None,
                    gas: Some(U256::from(4_000_000u64)),
                    gas_price: None,
                    value: None,
                    data: Some(Bytes(
                        hex::decode(ERC20_TOKEN_CONTRACT_CODE.trim()).unwrap(),
                    )),
                    nonce: None,
                    condition: None,
                },
                "",
            )
            .wait()
            .unwrap();

        let receipt = self
            .client
            .eth()
            .transaction_receipt(contract_tx_id)
            .wait()
            .unwrap()
            .unwrap();

        log::debug!(
            "Deploying the contract consumed {} gas",
            receipt.gas_used.expect("Gas used is present")
        );

        receipt.contract_address.unwrap()
    }

    pub fn get_contract_code(&self, address: Address) -> Bytes {
        self.client.eth().code(address, None).wait().unwrap()
    }

    pub fn get_contract_address(&self, txid: H256) -> Address {
        self.client
            .eth()
            .transaction_receipt(txid)
            .wait()
            .unwrap()
            .unwrap()
            .contract_address
            .unwrap()
    }

    pub fn mint_tokens(&self, contract: Address, amount: U256, to: Address) -> U256 {
        let function_identifier = "40c10f19";
        let address = format!("000000000000000000000000{}", hex::encode(to));
        let amount = format!("{:0>64}", format!("{:x}", amount));

        let payload = format!("{}{}{}", function_identifier, address, amount);

        self.send_data(contract, Some(Bytes(hex::decode(payload).unwrap())))
            .gas_used
            .expect("gas used is present")
    }

    pub fn token_balance_of(&self, contract: Address, address: Address) -> U256 {
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

    pub fn eth_balance_of(&self, address: Address) -> U256 {
        self.client.eth().balance(address, None).wait().unwrap()
    }

    pub fn send_data(&self, to: Address, data: Option<Bytes>) -> TransactionReceipt {
        let result_tx = self
            .client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: *PARITY_DEV_ACCOUNT,
                    to: Some(to),
                    gas: None,
                    gas_price: None,
                    value: None,
                    data,
                    nonce: None,
                    condition: None,
                },
                "",
            )
            .wait()
            .unwrap();

        let receipt = self
            .client
            .eth()
            .transaction_receipt(result_tx)
            .wait()
            .unwrap()
            .unwrap();

        log::debug!("Transaction Receipt: {:?}", receipt);

        receipt
    }

    pub fn deploy_htlc(&self, data: Bytes, value: U256) -> H256 {
        self.sign_and_send(|nonce, gas_price| UnsignedTransaction {
            nonce,
            gas_price,
            gas_limit: U256::from(500_000),
            to: None,
            value,
            data: Some(data.clone()),
        })
    }

    pub fn sign_and_send<T: Fn(U256, U256) -> UnsignedTransaction>(
        &self,
        transaction_fn: T,
    ) -> H256 {
        let gas_price = U256::from(100);

        let mut lock = self.nonce.lock().unwrap();

        let nonce = lock.deref_mut();

        let transaction = transaction_fn(*nonce, gas_price);

        let signed_transaction = self.wallet.sign(&transaction);

        let tx_id = self
            .client
            .eth()
            .send_raw_transaction(signed_transaction.into())
            .wait()
            .unwrap();

        // If we get this far, everything worked.
        // Update the nonce and release the lock.
        self.increment_nonce(nonce);

        tx_id
    }

    fn increment_nonce(&self, nonce: &mut U256) {
        let next_nonce = *nonce + U256::from(1);
        log::debug!("Nonce was incremented from {} to {}", nonce, next_nonce);
        *nonce = next_nonce;
    }
}
