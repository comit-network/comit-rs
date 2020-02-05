use crate::{
    asset,
    asset::ethereum::FromWei,
    db::Swap,
    ethereum::Bytes,
    swap_protocols::{
        ledger::{self, ethereum::ChainId},
        rfc003::{Accept, Request, SecretHash},
        HashFunction, Role, SwapId,
    },
    timestamp::Timestamp,
};
use bitcoin::{
    hashes::{sha256d, Hash},
    secp256k1,
};
use libp2p::PeerId;
use quickcheck::{Arbitrary, Gen};
use std::ops::Deref;
use uuid::Uuid;

/// Generic newtype that allows us to implement quickcheck::Arbitrary on foreign
/// types
#[derive(Clone, Debug, Copy)]
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
        let chain_id = ChainId::from(g.next_u32());

        Quickcheck(chain_id)
    }
}

impl Arbitrary for Quickcheck<asset::Bitcoin> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let amount = asset::Bitcoin::from_sat(g.next_u64());

        Quickcheck(amount)
    }
}

impl Arbitrary for Quickcheck<crate::ethereum::U256> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);
        let u256 = crate::ethereum::U256::from(bytes);

        Quickcheck(u256)
    }
}

impl Arbitrary for Quickcheck<bitcoin::BlockHash> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);

        match sha256d::Hash::from_slice(&bytes) {
            Ok(block_id) => Quickcheck(block_id.into()),
            Err(bitcoin::hashes::Error::InvalidLength(..)) => panic!("we always generate 32 bytes"),
        }
    }
}

impl Arbitrary for Quickcheck<crate::asset::Ether> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let u256 = *Quickcheck::<crate::ethereum::U256>::arbitrary(g);
        let ether_quantity = crate::asset::Ether::from_wei(u256);

        Quickcheck(ether_quantity)
    }
}

impl Arbitrary for Quickcheck<crate::asset::Erc20Quantity> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let u256 = *Quickcheck::<crate::ethereum::U256>::arbitrary(g);
        let erc20_quantity = crate::asset::Erc20Quantity::from_wei(u256);

        Quickcheck(erc20_quantity)
    }
}

impl Arbitrary for Quickcheck<crate::asset::Erc20> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let token_contract = *Quickcheck::<crate::ethereum::Address>::arbitrary(g);
        let quantity = Quickcheck::<crate::asset::Erc20Quantity>::arbitrary(g).0;
        let erc20_token = crate::asset::Erc20 {
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
        let public_key = crate::bitcoin::PublicKey::from_secret_key(
            &secp256k1::Secp256k1::signing_only(),
            &secret_key,
        );

        Quickcheck(public_key)
    }
}

impl Arbitrary for Quickcheck<crate::ethereum::Address> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 20]>::arbitrary(g);

        Quickcheck(crate::ethereum::Address::from(bytes))
    }
}

impl Arbitrary for Quickcheck<crate::ethereum::U128> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 16]>::arbitrary(g);

        Quickcheck(crate::ethereum::U128::from(&bytes))
    }
}

impl Arbitrary for Quickcheck<crate::ethereum::H256> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);

        Quickcheck(crate::ethereum::H256::from(&bytes))
    }
}

impl Arbitrary for Quickcheck<crate::ethereum::Transaction> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Quickcheck(crate::ethereum::Transaction {
            hash: *Quickcheck::<crate::ethereum::H256>::arbitrary(g),
            nonce: *Quickcheck::<crate::ethereum::U256>::arbitrary(g),
            block_hash: Option::<Quickcheck<crate::ethereum::H256>>::arbitrary(g).map(|i| i.0),
            block_number: Option::<Quickcheck<crate::ethereum::U256>>::arbitrary(g).map(|i| i.0),
            transaction_index: Option::<Quickcheck<crate::ethereum::U128>>::arbitrary(g)
                .map(|i| i.0),
            from: *Quickcheck::<crate::ethereum::H160>::arbitrary(g),
            to: Option::<Quickcheck<crate::ethereum::H160>>::arbitrary(g).map(|i| i.0),
            value: *Quickcheck::<crate::ethereum::U256>::arbitrary(g),
            gas_price: *Quickcheck::<crate::ethereum::U256>::arbitrary(g),
            gas: *Quickcheck::<crate::ethereum::U256>::arbitrary(g),
            input: Bytes(Arbitrary::arbitrary(g)),
        })
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
        Request<ledger::bitcoin::Regtest, ledger::Ethereum, asset::Bitcoin, crate::asset::Ether>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::bitcoin::Regtest,
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            beta_asset: Quickcheck::<crate::asset::Ether>::arbitrary(g).0,
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::bitcoin::Testnet, ledger::Ethereum, asset::Bitcoin, crate::asset::Ether>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::bitcoin::Testnet,
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            beta_asset: Quickcheck::<crate::asset::Ether>::arbitrary(g).0,
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::bitcoin::Mainnet, ledger::Ethereum, asset::Bitcoin, crate::asset::Ether>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::bitcoin::Mainnet,
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            beta_asset: Quickcheck::<crate::asset::Ether>::arbitrary(g).0,
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::Ethereum, ledger::bitcoin::Mainnet, crate::asset::Erc20, asset::Bitcoin>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            beta_ledger: ledger::bitcoin::Mainnet,
            alpha_asset: Quickcheck::<crate::asset::Erc20>::arbitrary(g).0,
            beta_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::Ethereum, ledger::bitcoin::Mainnet, crate::asset::Ether, asset::Bitcoin>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            beta_ledger: ledger::bitcoin::Mainnet,
            alpha_asset: Quickcheck::<crate::asset::Ether>::arbitrary(g).0,
            beta_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl Arbitrary
    for Quickcheck<
        Request<ledger::bitcoin::Mainnet, ledger::Ethereum, asset::Bitcoin, crate::asset::Erc20>,
    >
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Request {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger: ledger::bitcoin::Mainnet,
            beta_ledger: ledger::Ethereum {
                chain_id: *Quickcheck::<ChainId>::arbitrary(g),
            },
            alpha_asset: *Quickcheck::<asset::Bitcoin>::arbitrary(g),
            beta_asset: Quickcheck::<crate::asset::Erc20>::arbitrary(g).0,
            hash_function: *Quickcheck::<HashFunction>::arbitrary(g),
            alpha_ledger_refund_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_redeem_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
            alpha_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            beta_expiry: *Quickcheck::<Timestamp>::arbitrary(g),
            secret_hash: *Quickcheck::<SecretHash>::arbitrary(g),
        })
    }
}

impl<B: ledger::Bitcoin> Arbitrary for Quickcheck<Accept<B, ledger::Ethereum>> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Accept {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger_redeem_identity: *Quickcheck::<crate::bitcoin::PublicKey>::arbitrary(g),
            beta_ledger_refund_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
        })
    }
}

impl<B: ledger::Bitcoin> Arbitrary for Quickcheck<Accept<ledger::Ethereum, B>> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Quickcheck(Accept {
            swap_id: *Quickcheck::<SwapId>::arbitrary(g),
            alpha_ledger_redeem_identity: *Quickcheck::<crate::ethereum::Address>::arbitrary(g),
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
