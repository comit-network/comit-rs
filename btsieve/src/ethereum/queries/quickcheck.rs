use crate::web3::types::{Bytes, Transaction, H160, H256, U128, U256};
use ::quickcheck::Arbitrary;

#[derive(Clone, Debug)]
pub struct Quickcheck<I>(pub I);

impl From<Quickcheck<U128>> for U128 {
    fn from(source: Quickcheck<U128>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<U128> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 16];
        g.fill_bytes(&mut inner);

        Quickcheck(U128::from(&inner))
    }
}

impl From<Quickcheck<U256>> for U256 {
    fn from(source: Quickcheck<U256>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<U256> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 32];
        g.fill_bytes(&mut inner);

        Quickcheck(U256::from(&inner))
    }
}

impl From<Quickcheck<H160>> for H160 {
    fn from(source: Quickcheck<H160>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<H160> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 20];
        g.fill_bytes(&mut inner);

        Quickcheck(H160::from(&inner))
    }
}

impl From<Quickcheck<H256>> for H256 {
    fn from(source: Quickcheck<H256>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<H256> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 32];
        g.fill_bytes(&mut inner);

        Quickcheck(H256::from(&inner))
    }
}

impl From<Quickcheck<Transaction>> for Transaction {
    fn from(source: Quickcheck<Transaction>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<Transaction> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Quickcheck(Transaction {
            hash: <Quickcheck<H256> as Arbitrary>::arbitrary(g).into(),
            nonce: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            block_hash: Option::arbitrary(g)
                .map(|quickcheck_h256: Quickcheck<H256>| H256::from(quickcheck_h256)),
            block_number: Option::arbitrary(g)
                .map(|quickcheck_u256: Quickcheck<U256>| U256::from(quickcheck_u256)),
            transaction_index: Option::arbitrary(g)
                .map(|quickcheck_u128: Quickcheck<U128>| U128::from(quickcheck_u128)),
            from: <Quickcheck<H160> as Arbitrary>::arbitrary(g).into(),
            to: Option::arbitrary(g)
                .map(|quickcheck_h160: Quickcheck<H160>| H160::from(quickcheck_h160)),
            value: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            gas_price: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            gas: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            input: Bytes(Arbitrary::arbitrary(g)),
        })
    }
}
