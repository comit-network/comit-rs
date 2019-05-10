use crate::swap_protocols::{
    ledger::Ethereum,
    rfc003::{state_machine::HtlcParams, Timestamp},
};
use blockchain_contracts::ethereum::rfc003::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};
use ethereum_support::{web3::types::U256, Address, Bytes, Erc20Token, EtherQuantity, Network};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractDeploy {
    pub data: Bytes,
    pub amount: EtherQuantity,
    pub gas_limit: U256,
    pub network: Network,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CallContract {
    pub to: Address,
    pub data: Bytes,
    pub gas_limit: U256,
    pub network: Network,
    pub min_block_timestamp: Option<Timestamp>,
}

impl From<HtlcParams<Ethereum, EtherQuantity>> for ContractDeploy {
    fn from(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self {
        let htlc = EtherHtlc::from(htlc_params.clone());
        let gas_limit = htlc.deployment_gas_limit();

        ContractDeploy {
            data: htlc.into(),
            amount: htlc_params.asset,
            gas_limit,
            network: htlc_params.ledger.network,
        }
    }
}

impl From<HtlcParams<Ethereum, Erc20Token>> for ContractDeploy {
    fn from(htlc_params: HtlcParams<Ethereum, Erc20Token>) -> Self {
        let htlc = Erc20Htlc::from(htlc_params.clone());
        let gas_limit = htlc.deployment_gas_limit();

        ContractDeploy {
            data: htlc.into(),
            amount: EtherQuantity::zero(),
            gas_limit,
            network: htlc_params.ledger.network,
        }
    }
}
