use ethereum_support::{Address, Bytes, H256, U256};
use rlp::{Encodable, RlpStream};
use std::fmt;
use tiny_keccak::keccak256;

#[derive(Debug)]
pub struct UnsignedTransaction {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: Option<Address>,
    pub value: U256,
    pub data: Option<Bytes>,
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
