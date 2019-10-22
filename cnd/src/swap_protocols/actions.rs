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
    use bitcoin::{Address, Amount};
    use blockchain_contracts::bitcoin::witness::{PrimedInput, PrimedTransaction};

    #[derive(Debug, Clone, PartialEq)]
    pub struct SendToAddress {
        pub to: Address,
        pub amount: Amount,
        pub network: bitcoin::Network,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpendOutput {
        // Remember: One man's input is another man's output!
        pub output: PrimedInput,
        pub network: bitcoin::Network,
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeployContract {
        pub data: Bytes,
        pub amount: EtherQuantity,
        pub gas_limit: U256,
        pub network: Network,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CallContract {
        pub to: Address,
        pub data: Option<Bytes>,
        pub gas_limit: U256,
        pub network: Network,
        pub min_block_timestamp: Option<Timestamp>,
    }
}
