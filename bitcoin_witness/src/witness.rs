use bitcoin_support::{Hash160, Script};
use secp256k1_support::{PublicKey, SecretKey, ToPublicKey};

#[derive(Clone, Debug)]
pub enum Witness {
    Data(Vec<u8>),
    Signature(SecretKey),
    PublicKey(PublicKey),
    Bool(bool),
    PrevScript,
}

pub trait WitnessMethod {
    fn into_witness(self) -> Vec<Witness>;
    fn sequence(&self) -> u32;
    fn prev_script(&self) -> Script;
}

pub struct WitnessP2pkh(pub SecretKey);

impl WitnessMethod for WitnessP2pkh {
    fn into_witness(self) -> Vec<Witness> {
        vec![
            Witness::Signature(self.0),
            Witness::PublicKey(self.0.to_public_key()),
        ]
    }

    fn sequence(&self) -> u32 {
        super::SEQUENCE_ALLOW_NTIMELOCK_NO_RBF
    }

    fn prev_script(&self) -> Script {
        let public_key = self.0.to_public_key();
        let public_key_hash = Hash160::from_data(&public_key.serialize())[..].to_vec();
        // A standard p2wpkh locking script of:
        // 00 14 <public_key_hash>
        // becomes
        // 19 76 a9 14 <public_key_hash> 88 ac
        // in the unlocking script. See BIP 143.
        let mut prev_script = vec![0x76, 0xa9, 0x14];

        prev_script.append(&mut public_key_hash.clone());
        prev_script.push(0x88);
        prev_script.push(0xac);

        Script::from(prev_script)
    }
}

#[cfg(test)]
mod test {
    use bitcoin_support::PrivateKey;
    use bitcoin_support::Transaction;
    use bitcoin_support::serialize::deserialize;
    use std::str::FromStr;
    extern crate hex;
    use super::*;
    #[test]
    fn correct_prev_script() {
        let private_key =
            PrivateKey::from_str("L4r4Zn5sy3o5mjiAdezhThkU37mcdN4eGp4aeVM4ZpotGTcnWc6k").unwrap();
        let secret_key = private_key.secret_key();
        let witness_method = WitnessP2pkh(secret_key.clone());

        // Note: You might expect it to be a is_p2wpkh() but it shouldn't be.
        assert!(
            witness_method.prev_script().is_p2pkh(),
            "prev_script should be a p2pkh"
        );
    }

}
