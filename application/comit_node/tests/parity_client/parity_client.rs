use ethereum_support::{
    web3::{transports::Http, Web3},
    Address, Bytes, CallRequest, EthereumQuantity, Future, H256, TransactionRequest, U256,
};
use hex;

pub struct ParityClient {
    client: Web3<Http>,
}

lazy_static! {
    static ref PARITY_DEV_ACCOUNT: Address =
        "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();
}

const ERC20_TOKEN_CONTRACT_CODE: &'static str = include_str!("erc20_token_contract.asm.hex");

const PARITY_DEV_PASSWORD: &str = "";

impl ParityClient {
    pub fn new(client: Web3<Http>) -> Self {
        ParityClient { client }
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

    pub fn deploy_erc20_token_contract(&self) -> Address {
        let contract_tx_id = self
            .client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: PARITY_DEV_ACCOUNT.clone(),
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

        debug!("Deploying the contract consumed {} gas", receipt.gas_used);

        receipt.contract_address.unwrap()
    }

    pub fn get_contract_code(&self, address: Address) -> Bytes {
        self.client.eth().code(address, None).wait().unwrap()
    }

    pub fn get_contract_address(&self, txid: H256) -> Address {
        let receipt = self
            .client
            .eth()
            .transaction_receipt(txid)
            .wait()
            .unwrap()
            .unwrap();

        receipt.contract_address.unwrap()
    }

    pub fn mint_tokens(&self, contract: Address, amount: U256, to: Address) -> U256 {
        let function_identifier = "40c10f19";
        let address = format!("000000000000000000000000{}", hex::encode(to));
        let amount = format!("{:0>64}", format!("{:x}", amount));

        let payload = format!("{}{}{}", function_identifier, address, amount);

        self.send_data(contract, Some(Bytes(hex::decode(payload).unwrap())))
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

    pub fn send_data(&self, to: Address, data: Option<Bytes>) -> U256 {
        let result_tx = self
            .client
            .personal()
            .send_transaction(
                TransactionRequest {
                    from: PARITY_DEV_ACCOUNT.clone(),
                    to: Some(to),
                    gas: None,
                    gas_price: None,
                    value: None,
                    data: data,
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

        receipt.gas_used
    }
}
