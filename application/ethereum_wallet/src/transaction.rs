use ethereum_support::{Address, Bytes, H256, U256};
use hex;
use rlp::{Encodable, RlpStream};
use tiny_keccak::keccak256;

pub struct UnsignedTransaction {
    nonce: U256,
    gas_price: U256,
    gas_limit: U256,
    to: Option<Address>,
    value: U256,
    data: Option<Bytes>,
}

pub struct SignedTransaction<'a> {
    unsigned_transaction: &'a UnsignedTransaction,
    v: u8,
    signature: [u8; 64],
}

impl<'a> SignedTransaction<'a> {
    pub(crate) fn new(
        unsigned_transaction: &'a UnsignedTransaction,
        v: u8,
        signature: [u8; 64],
    ) -> Self {
        SignedTransaction {
            unsigned_transaction,
            v,
            signature,
        }
    }
}

impl<'a> Encodable for SignedTransaction<'a> {
    fn rlp_append(&self, stream: &mut RlpStream) {
        let r = &self.signature[0..32];
        let s = &self.signature[32..64];

        self.unsigned_transaction.rlp_append(stream);
        stream.append(&self.v);
        stream.append(&r);
        stream.append(&s);
    }
}

impl<'a> From<SignedTransaction<'a>> for Bytes {
    fn from(s: SignedTransaction) -> Self {
        let mut stream = RlpStream::new();

        stream.append(&s);

        let bytes = stream.as_raw();

        Bytes(bytes.to_vec())
    }
}

impl Encodable for UnsignedTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9);

        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);

        match self.to {
            Some(ref address) => s.append(address),
            None => s.append(&""),
        };

        s.append(&self.value);
        s.append(&self.data.clone().map(|b| b.0).unwrap_or([].to_vec()));
    }
}

impl UnsignedTransaction {
    pub fn new_contract_deployment<B: Into<Bytes>, GP: Into<U256>, V: Into<U256>, N: Into<U256>>(
        contract: B,
        gas_price: GP,
        value: V,
        nonce: N,
        extra_gas_limit: Option<u32>,
    ) -> Self {
        let contract_data = contract.into();
        let data_bytes = contract_data.0.len();
        let contract_creation_fee = 32000;
        let base_tx_fee = 21000;
        let gas_limit = contract_creation_fee + base_tx_fee + data_bytes * 200;

        let buffer: U256 = 10000.into();
        let buffered_gas_limit = U256::from(gas_limit)
            + buffer
            + extra_gas_limit.map(Into::into).unwrap_or(U256::from(0));

        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: buffered_gas_limit,
            to: None,
            value: value.into(),
            data: Some(contract_data),
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
        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: gas.into(),
            to: Some(to.into()),
            value: value.into(),
            data: None,
        }
    }

    pub fn new_contract_invocation<
        B: Into<Bytes>,
        A: Into<Address>,
        G: Into<U256>,
        GP: Into<U256>,
        V: Into<U256>,
        N: Into<U256>,
    >(
        data: B,
        to: A,
        gas: G,
        gas_price: GP,
        value: V,
        nonce: N,
    ) -> Self {
        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: gas.into(),
            to: Some(to.into()),
            value: value.into(),
            data: Some(data.into()),
        }
    }

    pub fn new_erc20_approval<
        TokenContract: Into<Address>,
        To: Into<Address>,
        Amount: Into<U256>,
        G: Into<U256>,
        GP: Into<U256>,
        N: Into<U256>,
    >(
        token_contract: TokenContract,
        to: To,
        amount: Amount,
        gas: G,
        gas_price: GP,
        nonce: N,
    ) -> Self {
        let function_identifier = "095ea7b3";
        let address = format!("000000000000000000000000{}", hex::encode(to.into()));
        let amount = format!("{:0>64}", format!("{:x}", amount.into()));

        let payload = format!("{}{}{}", function_identifier, address, amount);

        let data = Bytes::from(hex::decode(payload).unwrap());

        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: gas.into(),
            to: Some(token_contract.into()),
            value: U256::from(0),
            data: Some(data.into()),
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
    fn erc20_contract_allowance_should_have_correct_representation() {
        let transaction = UnsignedTransaction::new_erc20_approval(
            "03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c",
            "96984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e",
            1000,
            0,
            0,
            0,
        );

        assert_eq!(transaction.data, Some(Bytes(hex::decode("095ea7b300000000000000000000000096984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e00000000000000000000000000000000000000000000000000000000000003e8").unwrap())));
        assert_eq!(
            transaction.to,
            Some("03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c".into())
        );
    }

    #[test]
    fn gas_limit_is_computed_based_on_contract_size() {
        let transaction = UnsignedTransaction::new_contract_deployment(
            // contract occupying 5 bytes
            Bytes(vec![0_u8; 5]),
            0,
            0,
            0,
            // extra_gas_limit
            Some(10),
        );

        assert_eq!(
            transaction.gas_limit,
            U256::from(32000 + (5 * 200)) + 10000 + 10
        );
    }
}
