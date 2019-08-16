use crate::ethereum_helper::transaction::{SignedTransaction, UnsignedTransaction};
use blockchain_contracts::ethereum::to_ethereum_address::ToEthereumAddress;
use secp256k1::{Message, PublicKey, SecretKey};
use web3::types::Address;

pub trait Wallet: Send + Sync {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a>;
    fn address(&self) -> Address;
}

#[derive(Debug)]
pub struct InMemoryWallet {
    secret_key: SecretKey,
    public_key: PublicKey,
    chain_id: u8,
}

impl InMemoryWallet {
    pub fn new(secret_key: SecretKey, chain_id: u8) -> Self {
        InMemoryWallet {
            secret_key,
            public_key: secp256k1::PublicKey::from_secret_key(
                &*blockchain_contracts::SECP,
                &secret_key,
            ),
            chain_id,
        }
    }

    // https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md#specification
    fn chain_replay_protection_offset(&self) -> u8 {
        35 + self.chain_id * 2
    }
}

impl Wallet for InMemoryWallet {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a> {
        let hash: [u8; 32] = tx.hash(self.chain_id).into();
        // `from_slice` can be replaced with `from` once https://github.com/rust-bitcoin/rust-secp256k1/issues/106 is done
        let message = Message::from_slice(&hash).expect("Cannot fail as it is a [u8; 32]");
        let signature = blockchain_contracts::SECP.sign_recoverable(&message, &self.secret_key);

        let (rec_id, signature) = signature.serialize_compact();

        let v = rec_id.to_i32() as u8 + self.chain_replay_protection_offset();

        SignedTransaction::new(tx, v, signature)
    }

    fn address(&self) -> Address {
        self.public_key.to_ethereum_address()
    }
}
