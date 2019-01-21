use bitcoin_support::{Address, BitcoinQuantity};
use bitcoin_witness::{PrimedInput, PrimedTransaction};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendToAddress {
    pub to: Address,
    pub amount: BitcoinQuantity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendOutput {
    // Remember: One man's input is another man's output!
    // TODO: decide whether we want to serialize this directly
    pub output: PrimedInput,
}

impl SpendOutput {
    pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![self.output],
            locktime: 0,
            output_address: to_address,
        }
    }
}
