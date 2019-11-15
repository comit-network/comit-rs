use crate::web3::types::{Address, U256};
use rlp::RlpStream;
use tiny_keccak::{Hasher, Keccak};

pub trait CalculateContractAddress {
    fn calculate_contract_address(&self, nonce: &U256) -> Address;
}

impl CalculateContractAddress for Address {
    fn calculate_contract_address(&self, nonce: &U256) -> Address {
        let mut stream = RlpStream::new_list(2);

        let ethereum_address: &[u8] = self.as_ref();

        let raw_stream = stream.append(&ethereum_address).append(nonce).as_raw();
        let value = hash(raw_stream);

        let mut address = Address::default();
        address.assign_from_slice(&value[12..]);
        address
    }
}

fn hash(input: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];

    let mut hasher = Keccak::v256();
    hasher.update(input);
    hasher.finalize(&mut output);

    output
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
            Address::from_str("ad5768f87c7cb54477cb36d1fc9fdee740810661").unwrap()
        );

        let contract_address = address.calculate_contract_address(&U256::from(1));
        assert_eq!(
            contract_address,
            Address::from_str("994a1e7928556ba81b85bf3c665a3f4a0f0d4cd9").unwrap()
        );
    }
}
