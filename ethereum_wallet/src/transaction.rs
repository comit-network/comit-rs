use rlp::Encodable;
use rlp::RlpStream;
use tiny_keccak::keccak256;
use web3::types::Address;
use web3::types::Bytes;
use web3::types::H256;
use web3::types::U256;

pub struct Transaction {
    nonce: U256,
    gas_price: U256,
    gas: U256,
    to: Option<Address>,
    value: U256,
    data: Option<Bytes>,
}

impl Encodable for Transaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9);

        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);

        match self.to {
            Some(ref address) => s.append(address),
            None => s.append(&""),
        };

        s.append(&self.value);
        s.append(&self.data.clone().map(|b| b.0).unwrap_or([].to_vec()));
    }
}

impl Transaction {
    pub fn new_contract_deployment<
        B: Into<Bytes>,
        G: Into<U256>,
        GP: Into<U256>,
        V: Into<U256>,
        N: Into<U256>,
    >(
        contract: B,
        gas: G,
        gas_price: GP,
        value: V,
        nonce: N,
    ) -> Self {
        Transaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas: gas.into(),
            to: None,
            value: value.into(),
            data: Some(contract.into()),
        }
    }

    pub fn new_payment<
        A: Into<Address>,
        G: Into<U256>,
        GP: Into<U256>,
        V: Into<U256>,
        N: Into<U256>,
    >(
        to: A,
        gas: G,
        gas_price: GP,
        value: V,
        nonce: N,
    ) -> Self {
        Transaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas: gas.into(),
            to: Some(to.into()),
            value: value.into(),
            data: None,
        }
    }

    pub(crate) fn hash(&self, chain_id: u8) -> H256 {
        let mut stream = RlpStream::new();

        self.rlp_append(&mut stream);
        stream.append(&chain_id);
        stream.append(&0u8);
        stream.append(&0u8);

        let bytes = stream.as_raw();
        let tx_hash = keccak256(bytes);

        H256(tx_hash)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn contract_deployment_transaction_should_have_correct_binary_representation() {
        let tx = Transaction::new_contract_deployment(Bytes(Vec::new()), 500, 2, 10, 1);

        let mut stream = RlpStream::new();

        tx.rlp_append(&mut stream);
        stream.append(&1u8);
        stream.append(&0u8);
        stream.append(&0u8);

        let bytes = stream.as_raw();

        assert_eq!(bytes, &[203, 1, 2, 130, 1, 244, 128, 10, 128, 1, 128, 128]);
    }

    #[test]
    fn payment_transaction_should_have_correct_binary_representation() {
        let tx =
            Transaction::new_payment("147ba99ef89c152f8004e91999fee87bda6cbc3e", 500, 2, 10, 1);

        let mut stream = RlpStream::new();

        tx.rlp_append(&mut stream);
        stream.append(&1u8);
        stream.append(&0u8);
        stream.append(&0u8);

        let bytes = stream.as_raw();

        assert_eq!(
            bytes,
            &[
                223, 1, 2, 130, 1, 244, 148, 20, 123, 169, 158, 248, 156, 21, 47, 128, 4, 233, 25,
                153, 254, 232, 123, 218, 108, 188, 62, 10, 128, 1, 128, 128,
            ]
        );
    }

}
