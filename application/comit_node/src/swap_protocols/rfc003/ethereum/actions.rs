use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDeploy {
    pub data: Bytes,
    pub value: EtherQuantity,
    pub gas_limit: U256,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendTransaction {
    pub to: Address,
    pub data: Bytes,
    pub gas_limit: U256,
    pub value: EtherQuantity,
}

impl SendTransaction {
    pub fn serialize(&self, to: String) -> Result<String, ()> {
        unimplemented!()
    }
}
