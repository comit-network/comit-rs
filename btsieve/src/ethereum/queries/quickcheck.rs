use ::quickcheck::Arbitrary;
use ethereum_support::web3::types::{Bytes, Transaction, H160, H256, U128, U256};

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
            block_hash: <Option<Quickcheck<H256>> as Arbitrary>::arbitrary(g).map(H256::from),
            block_number: <Option<Quickcheck<U256>> as Arbitrary>::arbitrary(g).map(U256::from),
            transaction_index: <Option<Quickcheck<U128>> as Arbitrary>::arbitrary(g)
                .map(U128::from),
            from: <Quickcheck<H160> as Arbitrary>::arbitrary(g).into(),
            to: <Option<Quickcheck<H160>> as Arbitrary>::arbitrary(g).map(H160::from),
            value: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            gas_price: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            gas: <Quickcheck<U256> as Arbitrary>::arbitrary(g).into(),
            input: Bytes(Arbitrary::arbitrary(g)),
        })
    }
}
