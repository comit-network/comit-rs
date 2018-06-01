use Transaction;
use rlp::Encodable;
use rlp::RlpStream;
use secp256k1;
use secp256k1::ContextFlag;
use secp256k1::Secp256k1;
use secp256k1::SecretKey;
use web3::types::Bytes;

pub struct Wallet {
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

impl Wallet {
    pub fn new(private_key: [u8; 32], chain_id: u8) -> Result<Self, Error> {
        // TODO: Wrap / fork this library to make a more Rust-like interface
        // Idea: Encode the capabilities into generics so that an instance 'remembers' which capabilities it has and the compiler resolves the error handling for you.
        let context = Secp256k1::with_caps(ContextFlag::Full);

        let private_key = SecretKey::from_slice(&context, &private_key)?;

        Ok(Wallet {
            context,
            private_key,
            chain_id,
        })
    }

    pub fn create_signed_raw_transaction(&self, tx: &Transaction) -> Bytes {
        let hash: [u8; 32] = tx.hash(self.chain_id).into();

        let signature = match self.context
            .sign_recoverable(&hash.into(), &self.private_key)
        {
            Ok(signature) => signature,
            Err(_e) => panic!("Bug! Secp256k1 instance was constructed with wrong capabilities."),
        };

        let (rec_id, data) = signature.serialize_compact(&self.context);

        let r = &data[0..32];
        let s = &data[32..64];
        let v = (rec_id.to_i32() + self.chain_replay_protection_offset()) as u8;

        let mut stream = RlpStream::new();

        tx.rlp_append(&mut stream);
        stream.append(&v);
        stream.append(&r);
        stream.append(&s);

        let bytes = stream.as_raw();

        Bytes(bytes.to_vec())
    }

    fn chain_replay_protection_offset(&self) -> i32 {
        35 + self.chain_id as i32 * 2
    }
}
