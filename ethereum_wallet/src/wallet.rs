use UnsignedTransaction;
use secp256k1;
use secp256k1::ContextFlag;
use secp256k1::Secp256k1;
use secp256k1::SecretKey;
use transaction::SignedTransaction;

pub trait Wallet: Send + Sync {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a>;
}

pub struct InMemoryWallet {
    context: Secp256k1,
    private_key: SecretKey,
    chain_id: u8,
}

#[derive(Debug)]
pub enum Error {
    Crypto(secp256k1::Error),
}

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Self {
        Error::Crypto(e)
    }
}

impl InMemoryWallet {
    pub fn new(private_key: [u8; 32], chain_id: u8) -> Result<Self, Error> {
        // TODO: Wrap / fork this library to make a more Rust-like interface
        // Idea: Encode the capabilities into generics so that an instance 'remembers' which capabilities it has and the compiler resolves the error handling for you.
        let context = Secp256k1::with_caps(ContextFlag::Full);

        let private_key = SecretKey::from_slice(&context, &private_key)?;

        Ok(InMemoryWallet {
            context,
            private_key,
            chain_id,
        })
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
