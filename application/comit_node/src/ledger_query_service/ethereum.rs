use ethereum_support::web3::types::{Address, Bytes};

#[derive(Debug, Default, Clone, Serialize)]
pub struct EthereumQuery {
    pub from_address: Option<Address>,
    pub to_address: Option<Address>,
    pub is_contract_creation: Option<bool>,
    pub transaction_data: Option<Bytes>,
}
