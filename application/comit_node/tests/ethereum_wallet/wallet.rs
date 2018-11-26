use ethereum_support::{Address, ToEthereumAddress};
use ethereum_wallet::transaction::{SignedTransaction, UnsignedTransaction};
use secp256k1_support::{KeyPair, RecoverableSignature};

pub trait Wallet: Send + Sync {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a>;
    fn address(&self) -> Address;
}

#[derive(Debug)]
pub struct InMemoryWallet {
    keypair: KeyPair,
    chain_id: u8,
}

impl InMemoryWallet {
    pub fn new(keypair: KeyPair, chain_id: u8) -> Self {
        InMemoryWallet { keypair, chain_id }
    }

    pub fn address(&self) -> Address {
        self.keypair.public_key().to_ethereum_address()
    }

    // https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md#specification
    fn chain_replay_protection_offset(&self) -> u8 {
        35 + self.chain_id * 2
    }
}

impl Wallet for InMemoryWallet {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a> {
        let hash: [u8; 32] = tx.hash(self.chain_id).into();

        let signature = self.keypair.sign_ecdsa_recoverable(hash.into());

        let (rec_id, signature) = RecoverableSignature::serialize_compact(&signature);

        let v = rec_id.to_i32() as u8 + self.chain_replay_protection_offset();

        SignedTransaction::new(tx, v, signature)
    }

    fn address(&self) -> Address {
        self.keypair.public_key().to_ethereum_address()
    }
}
