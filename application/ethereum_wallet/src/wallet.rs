use secp256k1::ContextFlag;
use secp256k1::Secp256k1;
use secp256k1::SecretKey;
use {SignedTransaction, UnsignedTransaction};

pub trait Wallet: Send + Sync {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a>;
}

pub struct InMemoryWallet {
    context: Secp256k1,
    private_key: SecretKey,
    chain_id: u8,
}

impl InMemoryWallet {
    pub fn new(private_key: SecretKey, chain_id: u8) -> Self {
        let context = Secp256k1::with_caps(ContextFlag::Full);

        InMemoryWallet {
            context,
            private_key,
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

        let signature = match self.context
            .sign_recoverable(&hash.into(), &self.private_key)
        {
            Ok(signature) => signature,
            Err(_e) => panic!("Bug! Secp256k1 instance was constructed with wrong capabilities."),
        };

        let (rec_id, signature) = signature.serialize_compact(&self.context);

        let v = rec_id.to_i32() as u8 + self.chain_replay_protection_offset();

        SignedTransaction::new(tx, v, signature)
    }
}
