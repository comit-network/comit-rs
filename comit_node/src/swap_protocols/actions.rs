/// Common interface across all protocols supported by COMIT
///
/// This trait is intended to be implemented on an Actor's state and return the
/// actions which are currently available in a given state.
pub trait Actions {
    /// Different protocols have different kinds of requirements for actions.
    /// Hence they get to choose the type here.
    type ActionKind;

    fn actions(&self) -> Vec<Self::ActionKind>;
}

pub mod bitcoin {
    use bitcoin_support::{Address, BitcoinQuantity, Network};
    use bitcoin_witness::{PrimedInput, PrimedTransaction};
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct SendToAddress {
        pub to: Address,
        pub amount: BitcoinQuantity,
        pub network: Network,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpendOutput {
        // Remember: One man's input is another man's output!
        pub output: PrimedInput,
        pub network: Network,
    }

    impl SpendOutput {
        pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
            PrimedTransaction {
                inputs: vec![self.output],
                output_address: to_address,
            }
        }
    }
}

pub mod ethereum {
    use crate::swap_protocols::Timestamp;
    use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity, Network};
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct DeployContract {
        pub data: Bytes,
        pub amount: EtherQuantity,
        pub gas_limit: U256,
        pub network: Network,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct CallContract {
        pub to: Address,
        pub data: Option<Bytes>,
        pub gas_limit: U256,
        pub network: Network,
        pub min_block_timestamp: Option<Timestamp>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_support::{web3::types::U256, Address, Network};
    use std::str::FromStr;

    #[test]
    fn call_contract_serializes_correctly_to_json_with_none() {
        let addr = Address::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();
        let contract = ethereum::CallContract {
            to: addr,
            data: None,
            gas_limit: U256::from(1),
            network: Network::Ropsten,
            min_block_timestamp: None,
        };
        let serialized = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serialized,
            r#"{"to":"0x0a81e8be41b21f651a71aab1a85c6813b8bbccf8","gas_limit":"0x1","network":"ropsten"}"#,
        );
    }
}
