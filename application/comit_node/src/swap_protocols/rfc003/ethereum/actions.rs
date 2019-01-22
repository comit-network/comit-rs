use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity, Network};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDeploy {
    pub data: Bytes,
    pub amount: EtherQuantity,
    pub gas_limit: U256,
    pub network: Network,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendTransaction {
    pub to: Address,
    pub data: Bytes,
    pub amount: EtherQuantity,
    pub gas_limit: U256,
    pub network: Network,
}
