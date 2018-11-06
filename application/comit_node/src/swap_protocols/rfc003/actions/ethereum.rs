use ethereum_support::{Address, Bytes, EtherQuantity};
use swap_protocols::rfc003::Secret;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EtherDeploy {
    pub data: Bytes,
    pub value: EtherQuantity,
    pub gas_limit: u32,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRefund {
    pub contract_address: Address,
    pub execution_gas: u32,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRedeem {
    pub contract_address: Address,
    pub execution_gas: u32,
    pub data: Secret,
}
