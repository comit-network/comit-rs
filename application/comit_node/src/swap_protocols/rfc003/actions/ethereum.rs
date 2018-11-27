use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity};
use swap_protocols::rfc003::Secret;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct EtherDeploy {
    pub data: Bytes,
    pub value: EtherQuantity,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRefund {
    pub contract_address: Address,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRedeem {
    pub contract_address: Address,
    pub data: Secret,
    pub gas_limit: U256,
    pub gas_cost: U256,
}
