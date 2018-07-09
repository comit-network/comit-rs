use bitcoin::blockdata::script::Script;
use bitcoin_support::PubkeyHash;
use bitcoin_witness::{Witness, WitnessMethod, SEQUENCE_ALLOW_NTIMELOCK_NO_RBF};
use common_types::secret::Secret;
use secp256k1_support::{SecretKey, ToPublicKey};
use secret::SecretHash;

pub struct WitnessHtlcSecret {
    pub script: Script,
    pub secret_key: SecretKey,
    pub secret: Secret,
}

#[derive(Debug)]
pub enum InvalidWitness {
    WrongSecret {
        got: SecretHash,
        expected: SecretHash,
    },
    WrongSecretKey {
        got: PubkeyHash,
        expected: PubkeyHash,
    },
}

impl WitnessHtlcSecret {
    pub fn validate(
        &self,
        expected_secret_hash: &SecretHash,
        expected_pubkey_hash: &PubkeyHash,
    ) -> Result<(), InvalidWitness> {
        let got_pubkey_hash: PubkeyHash = self.secret_key.to_public_key().into();
        let got_secret_hash = self.secret.hash();

        if *expected_secret_hash != got_secret_hash {
            return Err(InvalidWitness::WrongSecret {
                got: got_secret_hash,
                expected: expected_secret_hash.clone(),
            });
        }

        if *expected_pubkey_hash != got_pubkey_hash {
            return Err(InvalidWitness::WrongSecretKey {
                got: got_pubkey_hash,
                expected: expected_pubkey_hash.clone(),
            });
        }

        Ok(())
    }
}

impl WitnessMethod for WitnessHtlcSecret {
    fn into_witness(self) -> Vec<Witness> {
        vec![
            Witness::Signature(self.secret_key),
            Witness::PublicKey(self.secret_key.to_public_key()),
            Witness::Data(self.secret.raw_secret().to_vec()),
            Witness::Bool(true),
            Witness::PrevScript,
        ]
    }

    fn sequence(&self) -> u32 {
        SEQUENCE_ALLOW_NTIMELOCK_NO_RBF
    }

    fn prev_script(&self) -> Script {
        self.script.clone()
    }
}

pub struct WitnessHtlcTimeout {
    pub script: Script,
    pub secret_key: SecretKey,
    pub sequence: u32,
}

impl WitnessMethod for WitnessHtlcTimeout {
    fn into_witness(self) -> Vec<Witness> {
        vec![
            Witness::Signature(self.secret_key),
            Witness::PublicKey(self.secret_key.to_public_key()),
            Witness::Bool(false),
            Witness::PrevScript,
        ]
    }

    fn sequence(&self) -> u32 {
        self.sequence
    }

    fn prev_script(&self) -> Script {
        self.script.clone()
    }
}

impl WitnessHtlcTimeout {
    pub fn validate(&self, expected_pubkey_hash: &PubkeyHash) -> Result<(), InvalidWitness> {
        let got_pubkey_hash: PubkeyHash = self.secret_key.to_public_key().into();

        if *expected_pubkey_hash != got_pubkey_hash {
            return Err(InvalidWitness::WrongSecretKey {
                got: got_pubkey_hash,
                expected: expected_pubkey_hash.clone(),
            });
        }

        Ok(())
    }
}
