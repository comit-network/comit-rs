use crate::swap_protocols::{
    ledger::{self, ethereum::ChainId},
    rfc003::{Accept, Request, SecretHash},
    HashFunction, SwapId, Timestamp,
};
use quickcheck::{Arbitrary, Gen};
use std::ops::Deref;
use uuid::Uuid;

/// Generic newtype that allows us to implement quickcheck::Arbitrary on foreign
/// types
#[derive(Clone, Debug)]
pub struct Quickcheck<I>(pub I);

impl<I> Deref for Quickcheck<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Arbitrary for Quickcheck<SwapId> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 16];
        g.fill_bytes(&mut bytes);

        Quickcheck(SwapId::from(Uuid::from_bytes(bytes)))
    }
}

impl Arbitrary for Quickcheck<bitcoin::Network> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let network = match g.next_u32() % 3 {
            0 => bitcoin::Network::Bitcoin,
            1 => bitcoin::Network::Testnet,
            2 => bitcoin::Network::Regtest,
            _ => unreachable!(),
        };

        Quickcheck(network)
    }
}

impl Arbitrary for Quickcheck<ChainId> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(ChainId::new(g.next_u32()))
    }
}

impl Arbitrary for Quickcheck<bitcoin::Amount> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(bitcoin::Amount::from_sat(g.next_u64()))
    }
}

impl Arbitrary for Quickcheck<ethereum_support::U256> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        Quickcheck(ethereum_support::U256::from(bytes))
    }
}

impl Arbitrary for Quickcheck<ethereum_support::EtherQuantity> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(ethereum_support::EtherQuantity::from_wei(*Quickcheck::<
            ethereum_support::U256,
        >::arbitrary(
            g
        )))
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Erc20Quantity> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(ethereum_support::Erc20Quantity(*Quickcheck::<
            ethereum_support::U256,
        >::arbitrary(g)))
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Erc20Token> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(ethereum_support::Erc20Token {
            token_contract: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            quantity: *Quickcheck::<ethereum_support::Erc20Quantity>::arbitrary(g),
        })
    }
}

impl Arbitrary for Quickcheck<HashFunction> {
    fn arbitrary<G: Gen>(_g: &mut G) -> Self {
        Quickcheck(HashFunction::Sha256)
    }
}

impl Arbitrary for Quickcheck<crate::bitcoin::PublicKey> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&bytes)
            .expect("all bytes are a valid secret key");

        Quickcheck(crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_key,
        ))
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Address> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 20];
        g.fill_bytes(&mut bytes);

        Quickcheck(ethereum_support::Address::from(bytes))
    }
}

impl Arbitrary for Quickcheck<Timestamp> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Timestamp::from(g.next_u32()))
    }
}

impl Arbitrary for Quickcheck<SecretHash> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        Quickcheck(SecretHash::from(bytes))
    }
}

impl Arbitrary
    for Quickcheck<
        Request<
            ledger::Bitcoin,
            ledger::Ethereum,
            bitcoin::Amount,
            ethereum_support::EtherQuantity,
        >,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Bitcoin {
                network: *Quickcheck::<bitcoin::Network>::arbitrary(g),
            },
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<bitcoin::Amount>::arbitrary(g),
            beta_asset: *Quickcheck::<ethereum_support::EtherQuantity>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::Ethereum, ledger::Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            beta_ledger: ledger::Bitcoin {
                network: *Quickcheck::<bitcoin::Network>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<ethereum_support::Erc20Token>::arbitrary(g),
            beta_asset: *Quickcheck::<bitcoin::Amount>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<
            ledger::Ethereum,
            ledger::Bitcoin,
            ethereum_support::EtherQuantity,
            bitcoin::Amount,
        >,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            beta_ledger: ledger::Bitcoin {
                network: *Quickcheck::<bitcoin::Network>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<ethereum_support::EtherQuantity>::arbitrary(g),
            beta_asset: *Quickcheck::<bitcoin::Amount>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::Bitcoin, ledger::Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Bitcoin {
                network: *Quickcheck::<bitcoin::Network>::arbitrary(g),
            },
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<bitcoin::Amount>::arbitrary(g),
            beta_asset: *Quickcheck::<ethereum_support::Erc20Token>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary for Quickcheck<Accept<ledger::Bitcoin, ledger::Ethereum>> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Accept {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_refund_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
        })
    }
}

impl Arbitrary for Quickcheck<Accept<ledger::Ethereum, ledger::Bitcoin>> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Accept {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger_redeem_identity: *Quickcheck::<ethereum_support::Address>::arbitrary(g),
            beta_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
        })
    }
}
