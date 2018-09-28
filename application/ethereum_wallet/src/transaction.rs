use ethereum_support::{Address, Bytes, H256, U256};
use hex;
use rlp::{Encodable, RlpStream};
use std::fmt;
use tiny_keccak::keccak256;

const CONTRACT_CREATION_FEE: usize = 32_000;
const BASE_TX_FEE: usize = 21_000;
const GAS_COST_PER_BYTE: usize = 200;
const GAS_BUFFER: usize = 10_000;

#[derive(Debug)]
pub struct UnsignedTransaction {
    nonce: U256,
    gas_price: U256,
    gas_limit: U256,
    to: Option<Address>,
    value: U256,
    data: Option<Bytes>,
}

struct Signature([u8; 64]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&&self.0[..], f)
    }
}

#[derive(Debug)]
pub struct SignedTransaction<'a> {
    unsigned_transaction: &'a UnsignedTransaction,
    v: u8,
    signature: Signature,
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
            signature: Signature(signature),
        }
    }
}

impl<'a> Encodable for SignedTransaction<'a> {
    fn rlp_append(&self, stream: &mut RlpStream) {
        let r = &self.signature.0[0..32];
        let s = &self.signature.0[32..64];

        stream
            .append_internal(self.unsigned_transaction)
            .append(&self.v)
            .append(&r)
            .append(&s);
    }
}

impl<'a> From<SignedTransaction<'a>> for Bytes {
    fn from(s: SignedTransaction) -> Self {
        let mut stream = RlpStream::new();

        let bytes = stream.append(&s).as_raw();

        Bytes(bytes.to_vec())
    }
}

impl Encodable for UnsignedTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9)
            .append(&self.nonce)
            .append(&self.gas_price)
            .append(&self.gas_limit);

        match self.to {
            Some(ref address) => s.append(address),
            None => s.append(&""),
        };

        s.append(&self.value).append(
            &self
                .data
                .clone()
                .map(|b| b.0)
                .unwrap_or_else(|| [].to_vec()),
        );
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
        let gas_limit = CONTRACT_CREATION_FEE + BASE_TX_FEE + data_bytes * GAS_COST_PER_BYTE;
        let buffered_gas_limit = U256::from(gas_limit)
            + U256::from(GAS_BUFFER)
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

    pub fn new_payment<A: Into<Address>, GP: Into<U256>, V: Into<U256>, N: Into<U256>>(
        to: A,
        gas_price: GP,
        value: V,
        nonce: N,
        extra_gas_limit: Option<u32>,
    ) -> Self {
        let extra_gas_limit = extra_gas_limit.map(Into::into).unwrap_or(U256::from(0));

        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: U256::from(BASE_TX_FEE) + extra_gas_limit,
            to: Some(to.into()),
            value: value.into(),
            data: None,
        }
    }

    pub fn new_contract_invocation<
        B: Into<Bytes>,
        A: Into<Address>,
        GL: Into<U256>,
        GP: Into<U256>,
        V: Into<U256>,
        N: Into<U256>,
    >(
        data: B,
        to: A,
        gas_limit: GL,
        gas_price: GP,
        value: V,
        nonce: N,
    ) -> Self {
        UnsignedTransaction {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: gas_limit.into(),
            to: Some(to.into()),
            value: value.into(),
            data: Some(data.into()),
        }
    }

    pub fn new_erc20_approval<
        TokenContract: Into<Address>,
        To: Into<Address>,
        Amount: Into<U256>,
        GP: Into<U256>,
        N: Into<U256>,
    >(
        token_contract: TokenContract,
        to: To,
        amount: Amount,
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
            gas_limit: 200_000.into(),
            to: Some(token_contract.into()),
            value: U256::from(0),
            data: Some(data),
        }
    }

    pub(crate) fn hash(&self, chain_id: u8) -> H256 {
        let mut stream = RlpStream::new();
        let bytes = stream
            .append_internal(self)
            .append(&chain_id)
            .append(&0u8)
            .append(&0u8)
            .as_raw();

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
        );

        assert_eq!(transaction.data, Some(Bytes(hex::decode("095ea7b300000000000000000000000096984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e00000000000000000000000000000000000000000000000000000000000003e8").unwrap())));
        assert_eq!(
            transaction.to,
            Some("03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c".into())
        );
    }

    #[test]
    fn gas_limit_is_computed_based_on_contract_size() {
        let extra_gas_limit = 10;

        let transaction = UnsignedTransaction::new_contract_deployment(
            // contract occupying 5 bytes
            Bytes(vec![0_u8; 5]),
            0,
            0,
            0,
            Some(extra_gas_limit),
        );

        assert_eq!(
            transaction.gas_limit,
            U256::from(CONTRACT_CREATION_FEE + BASE_TX_FEE + (5 * GAS_COST_PER_BYTE))
                + GAS_BUFFER
                + extra_gas_limit
        );
    }
}
