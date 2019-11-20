use crate::{
    db::Swap,
    swap_protocols::{
        ledger::{self, ethereum::ChainId},
        rfc003::{Accept, Request, SecretHash},
        HashFunction, Role, SwapId, Timestamp,
    },
};
use libp2p::PeerId;
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

macro_rules! impl_arbitrary_for_byte_array {
    ([u8; $length:expr]) => {
        impl Arbitrary for Quickcheck<[u8; $length]> {
            fn arbitrary<G: Gen>(g: &mut G) -> Self {
                let mut bytes = [0u8; $length];
                g.fill_bytes(&mut bytes);

                Quickcheck(bytes)
            }
        }
    };
}

impl_arbitrary_for_byte_array!([u8; 16]);
impl_arbitrary_for_byte_array!([u8; 20]);
impl_arbitrary_for_byte_array!([u8; 32]);

impl Arbitrary for Quickcheck<SwapId> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 16]>::arbitrary(g);
        let uuid = Uuid::from_bytes(bytes);
        let swap_id = SwapId::from(uuid);

        Quickcheck(swap_id)
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
        let chain_id = ChainId::new(g.next_u32());

        Quickcheck(chain_id)
    }
}

impl Arbitrary for Quickcheck<bitcoin::Amount> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let amount = bitcoin::Amount::from_sat(g.next_u64());

        Quickcheck(amount)
    }
}

impl Arbitrary for Quickcheck<ethereum_support::U256> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);
        let u256 = ethereum_support::U256::from(bytes);

        Quickcheck(u256)
    }
}

impl Arbitrary for Quickcheck<ethereum_support::EtherQuantity> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let u256 = *Quickcheck::<ethereum_support::U256>::arbitrary(g);
        let ether_quantity = ethereum_support::EtherQuantity::from_wei(u256);

        Quickcheck(ether_quantity)
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Erc20Quantity> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let u256 = *Quickcheck::<ethereum_support::U256>::arbitrary(g);
        let erc20_quantity = ethereum_support::Erc20Quantity(u256);

        Quickcheck(erc20_quantity)
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Erc20Token> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let token_contract = *Quickcheck::<ethereum_support::Address>::arbitrary(g);
        let quantity = *Quickcheck::<ethereum_support::Erc20Quantity>::arbitrary(g);
        let erc20_token = ethereum_support::Erc20Token {
            token_contract,
            quantity,
        };

        Quickcheck(erc20_token)
    }
}

impl Arbitrary for Quickcheck<HashFunction> {
    fn arbitrary<G: Gen>(_g: &mut G) -> Self {
        Quickcheck(HashFunction::Sha256)
    }
}

impl Arbitrary for Quickcheck<crate::bitcoin::PublicKey> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);
        let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&bytes)
            .expect("all bytes are a valid secret key");
        let public_key = crate::bitcoin::PublicKey::from_secret_key(&*crate::SECP, &secret_key);

        Quickcheck(public_key)
    }
}

impl Arbitrary for Quickcheck<ethereum_support::Address> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 20]>::arbitrary(g);

        Quickcheck(ethereum_support::Address::from(bytes))
    }
}

impl Arbitrary for Quickcheck<Timestamp> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let timestamp = Timestamp::from(g.next_u32());

        Quickcheck(timestamp)
    }
}

impl Arbitrary for Quickcheck<SecretHash> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);

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

impl Arbitrary for Quickcheck<Role> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let role = match g.next_u32() % 2 {
            0 => Role::Alice,
            1 => Role::Bob,
            _ => unreachable!(),
        };

        Quickcheck(role)
    }
}

impl Arbitrary for Quickcheck<PeerId> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);
        let secret_key = libp2p::identity::secp256k1::SecretKey::from_bytes(bytes)
            .expect("any 32 bytes are a valid secret key");
        let keypair = libp2p::identity::secp256k1::Keypair::from(secret_key);
        let public_key = keypair.public().clone();
        let public_key = libp2p::core::PublicKey::Secp256k1(public_key);
        let peer_id = PeerId::from_public_key(public_key);

        Quickcheck(peer_id)
    }
}

impl Arbitrary for Quickcheck<Swap> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Swap {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            role: *Quickcheck::<Role>::arbitrary(g),
            counterparty: Quickcheck::<PeerId>::arbitrary(g).0,
        })
    }
}
