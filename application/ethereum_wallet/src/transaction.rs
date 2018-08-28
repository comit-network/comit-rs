use ethereum_support::{Address, Bytes, H256, U256};
use rlp::{Encodable, RlpStream};
use tiny_keccak::keccak256;

pub struct UnsignedTransaction {
    nonce: U256,
    gas_price: U256,
    gas: U256,
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
        s.append(&self.gas);

        match self.to {
            Some(ref address) => s.append(address),
            None => s.append(&""),
        };

        s.append(&self.value);
        s.append(&self.data.clone().map(|b| b.0).unwrap_or([].to_vec()));
    }
}

impl UnsignedTransaction {
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
        UnsignedTransaction {
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
        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas: gas.into(),
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
            gas: gas.into(),
            to: Some(to.into()),
            value: value.into(),
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
    use hex::FromHex;
    use secp256k1_support::KeyPair;
    use wallet::Wallet;
    use InMemoryWallet;

    #[test]
    fn contract_deployment_transaction_should_have_correct_binary_representation() {
        let tx = UnsignedTransaction::new_contract_deployment(Bytes(Vec::new()), 500, 2, 10, 1);

        let mut stream = RlpStream::new();

        tx.rlp_append(&mut stream);
        stream.append(&1u8);
        stream.append(&0u8);
        stream.append(&0u8);

        let bytes = stream.as_raw();

        assert_eq!(bytes, &[203, 1, 2, 130, 1, 244, 128, 10, 128, 1, 128, 128]);
    }

    #[test]
    fn contract_invocation_transaction_should_have_correct_binary_representation() {
        let tx = UnsignedTransaction::new_contract_invocation(
            Bytes(Vec::new()),
            "147ba99ef89c152f8004e91999fee87bda6cbc3e",
            500,
            2,
            0,
            1,
        );

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
                153, 254, 232, 123, 218, 108, 188, 62, 128, 128, 1, 128, 128
            ]
        );
    }

    #[test]
    fn payment_transaction_should_have_correct_binary_representation() {
        let tx = UnsignedTransaction::new_payment(
            "147ba99ef89c152f8004e91999fee87bda6cbc3e",
            500,
            2,
            10,
            1,
        );

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

    #[test]
    fn signed_transaction_should_have_correct_binary_representation() {
        let secret_key_data = <[u8; 32]>::from_hex(
            "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
        ).unwrap();
        let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();
        let account0 = InMemoryWallet::new(keypair, 1);

        let tx = UnsignedTransaction::new_payment(
            "147ba99ef89c152f8004e91999fee87bda6cbc3e",
            500,
            2,
            10,
            1,
        );

        let transaction = account0.sign(&tx);

        let bytes: Bytes = transaction.into();
        let bytes = bytes.0;

        assert_eq!(
            bytes,
            vec![
                248, 95, 1, 2, 130, 1, 244, 148, 20, 123, 169, 158, 248, 156, 21, 47, 128, 4, 233,
                25, 153, 254, 232, 123, 218, 108, 188, 62, 10, 128, 37, 160, 28, 83, 76, 32, 152,
                243, 119, 249, 92, 41, 113, 205, 218, 84, 153, 100, 194, 227, 142, 156, 175, 193,
                100, 142, 204, 2, 237, 132, 47, 44, 156, 101, 160, 3, 102, 136, 243, 157, 29, 196,
                161, 44, 128, 172, 193, 117, 230, 52, 200, 119, 125, 10, 192, 190, 228, 153, 205,
                209, 81, 123, 160, 70, 77, 10, 229,
            ]
        );
    }
}
