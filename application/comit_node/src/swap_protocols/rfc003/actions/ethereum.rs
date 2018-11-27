use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity};
use swap_protocols::rfc003::Secret;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct EtherDeploy {
    pub data: Bytes,
    pub value: EtherQuantity,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl EtherDeploy {
    pub fn new(data: Bytes, value: EtherQuantity) -> EtherDeploy {
        EtherDeploy {
            data,
            value,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRefund {
    pub to_address: Address,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl EtherRefund {
    pub fn new(to_address: Address) -> EtherRefund {
        EtherRefund {
            to_address,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherRedeem {
    pub to_address: Address,
    pub data: Secret,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl EtherRedeem {
    pub fn new(to_address: Address, data: Secret) -> EtherRedeem {
        EtherRedeem {
            to_address,
            data,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Erc20Deploy {
    pub data: Bytes,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl Erc20Deploy {
    pub fn new(data: Bytes) -> Erc20Deploy {
        Erc20Deploy {
            data,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Erc20Fund {
    pub to_address: Address,
    pub data: Bytes,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl Erc20Fund {
    pub fn new(to_address: Address, data: Bytes) -> Erc20Fund {
        Erc20Fund {
            to_address,
            data,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Refund {
    pub to_address: Address,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl Erc20Refund {
    pub fn new(to_address: Address) -> Erc20Refund {
        Erc20Refund {
            to_address,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Redeem {
    pub to_address: Address,
    pub data: Secret,
    pub gas_limit: U256,
    pub gas_cost: U256,
}

impl Erc20Redeem {
    pub fn new(to_address: Address, data: Secret) -> Erc20Redeem {
        Erc20Redeem {
            to_address,
            data,
            gas_limit: 42.into(), //TODO come up with correct gas limit
            gas_cost: 42.into(),  //TODO come up with correct gas cost
        }
    }
}
