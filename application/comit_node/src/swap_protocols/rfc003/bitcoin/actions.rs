use bitcoin_support::{Address, BitcoinQuantity, Network};
use bitcoin_witness::{PrimedInput, PrimedTransaction};

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
