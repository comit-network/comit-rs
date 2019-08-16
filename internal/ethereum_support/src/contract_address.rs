use crate::web3::types::{Address, U256};
use rlp::RlpStream;

pub trait CalculateContractAddress {
    fn calculate_contract_address(&self, nonce: &U256) -> Address;
}

impl CalculateContractAddress for Address {
    fn calculate_contract_address(&self, nonce: &U256) -> Address {
        let mut stream = RlpStream::new_list(2);

        let ethereum_address: &[u8] = self.as_ref();

        let raw_stream = stream.append(&ethereum_address).append(nonce).as_raw();

        let value = tiny_keccak::keccak256(raw_stream);

        let mut address = Address::default();
        address.copy_from_slice(&value[12..]);
        address
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::web3::types::Address;
    use std::str::FromStr;

    #[test]
    fn given_an_address_and_a_nonce_should_give_contract_address() {
        let address = Address::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();

        let contract_address = address.calculate_contract_address(&U256::from(0));
        assert_eq!(
            contract_address,
            "ad5768f87c7cb54477cb36d1fc9fdee740810661".into()
        );

        let contract_address = address.calculate_contract_address(&U256::from(1));
        assert_eq!(
            contract_address,
            "994a1e7928556ba81b85bf3c665a3f4a0f0d4cd9".into()
        );
    }
}
