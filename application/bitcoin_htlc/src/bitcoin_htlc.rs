use bitcoin::blockdata::{
    opcodes::All::{OP_NOP3 as OP_CHECKSEQUENCEVERIFY, *},
    script::Builder,
};
use bitcoin_support::{Address, Network, PubkeyHash, Script};
use bitcoin_witness::{UnlockParameters, Witness, SEQUENCE_ALLOW_NTIMELOCK_NO_RBF};
use secp256k1_support::KeyPair;
use secret::{Secret, SecretHash};

#[derive(Debug)]
pub enum UnlockingError {
    WrongSecret {
        got: SecretHash,
        expected: SecretHash,
    },
    WrongKeyPair {
        keypair: KeyPair,
        got: PubkeyHash,
        expected: PubkeyHash,
    },
}

// Create BTC HTLC
// Returns P2WSH address
// Input:
// - BTC address of the exchange to receive the funds (exchange_success_address)
// - BTC timeout
// - BTC amount
// - hashed secret

#[derive(Clone, Debug)]
pub struct Htlc {
    recipient_success_pubkey_hash: PubkeyHash,
    sender_refund_pubkey_hash: PubkeyHash,
    secret_hash: SecretHash,
    relative_timelock: u32,
    script: Script,
}

impl Htlc {
    pub fn new<
        RecipientSuccessPubkeyHash: Into<PubkeyHash>,
        SenderRefundPubkeyHash: Into<PubkeyHash>,
    >(
        recipient_success_pubkey_hash: RecipientSuccessPubkeyHash,
        sender_refund_pubkey_hash: SenderRefundPubkeyHash,
        secret_hash: SecretHash,
        relative_timelock: u32,
    ) -> Htlc {
        let recipient_success_pubkey_hash = recipient_success_pubkey_hash.into();
        let sender_refund_pubkey_hash = sender_refund_pubkey_hash.into();
        let script = create_htlc(
            &recipient_success_pubkey_hash,
            &sender_refund_pubkey_hash,
            &secret_hash.0,
            relative_timelock,
        );

        Htlc {
            recipient_success_pubkey_hash,
            sender_refund_pubkey_hash,
            secret_hash,
            relative_timelock,
            script,
        }
    }

    pub fn script(&self) -> &Script {
        &self.script
    }

    pub fn compute_address(&self, network: Network) -> Address {
        Address::p2wsh(&self.script, network)
    }

    pub fn can_be_unlocked_with(
        &self,
        got_secret: &Secret,
        got_keypair: &KeyPair,
    ) -> Result<(), UnlockingError> {
        let got_pubkey_hash: PubkeyHash = got_keypair.public_key().clone().into();
        let got_secret_hash = got_secret.hash();
        let expected_pubkey_hash = &self.recipient_success_pubkey_hash;
        let expected_secret_hash = &self.secret_hash;

        if *expected_secret_hash != got_secret_hash {
            return Err(UnlockingError::WrongSecret {
                got: got_secret_hash,
                expected: expected_secret_hash.clone(),
            });
        }

        if *expected_pubkey_hash != got_pubkey_hash {
            return Err(UnlockingError::WrongKeyPair {
                keypair: got_keypair.clone(),
                got: got_pubkey_hash,
                expected: expected_pubkey_hash.clone(),
            });
        }

        Ok(())
    }

    pub fn unlock_with_secret(&self, keypair: KeyPair, secret: Secret) -> UnlockParameters {
        let public_key = keypair.public_key().clone();
        UnlockParameters {
            witness: vec![
                Witness::Signature(keypair),
                Witness::PublicKey(public_key),
                Witness::Data(secret.raw_secret().to_vec()),
                Witness::Bool(true),
                Witness::PrevScript,
            ],
            sequence: SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
            prev_script: self.script.clone(),
        }
    }

    pub fn unlock_after_timeout(&self, keypair: KeyPair) -> UnlockParameters {
        let public_key = keypair.public_key().clone();
        UnlockParameters {
            witness: vec![
                Witness::Signature(keypair),
                Witness::PublicKey(public_key),
                Witness::Bool(false),
                Witness::PrevScript,
            ],
            sequence: self.relative_timelock,
            prev_script: self.script.clone(),
        }
    }
}

fn create_htlc(
    recipient_pubkey_hash: &PubkeyHash,
    sender_pubkey_hash: &PubkeyHash,
    secret_hash: &Vec<u8>,
    redeem_block_height: u32,
) -> Script {
    let script = Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(secret_hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(recipient_pubkey_hash.as_ref())
        .push_opcode(OP_ELSE)
        .push_int(redeem_block_height as i64)
        .push_opcode(OP_CHECKSEQUENCEVERIFY)
        .push_opcode(OP_DROP)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(sender_pubkey_hash.as_ref())
        .push_opcode(OP_ENDIF)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    debug!("BTC HTLC: {}", script);
    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    // Secret: 12345678901234567890123456789012
    // Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

    // Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
    // Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
    // Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

    // Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
    // Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
    // Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

    // htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b27576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
    // sha256 of htlc script: 82badc8d1175d1c7ecfceb67a6b8d24fa51718beb594002c7cd9ca1da706b4ef

    #[test]
    fn given_a_vec_u8_pubkey_hash_return_htlc_redeem_script() {
        let recipient_pubkey_hash: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let sender_pubkey_hash: Vec<u8> =
            hex::decode("1925a274ac004373bb5429553bdb55c40e57b124").unwrap();

        let recipient_pubkey_hash = PubkeyHash::from(&recipient_pubkey_hash[..]);
        let sender_pubkey_hash = PubkeyHash::from(&sender_pubkey_hash[..]);

        let secret_hash: Vec<u8> = hex::decode(
            "51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c",
        ).unwrap();

        let htlc = Htlc::new(
            recipient_pubkey_hash,
            sender_pubkey_hash,
            SecretHash(secret_hash),
            900,
        );

        assert_eq!(
            htlc.script.into_vec(),
            hex::decode(
                "63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f2345\
                 1af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f\
                 567028403b27576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac"
            ).unwrap()
        );
    }

    #[test]
    fn given_an_htlc_redeem_script_return_p2wsh() {
        let recipient_pubkey_hash: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let sender_pubkey_hash: Vec<u8> =
            hex::decode("1925a274ac004373bb5429553bdb55c40e57b124").unwrap();

        let recipient_pubkey_hash = PubkeyHash::from(&recipient_pubkey_hash[..]);
        let sender_pubkey_hash = PubkeyHash::from(&sender_pubkey_hash[..]);

        let secret_hash: Vec<u8> = hex::decode(
            "51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c",
        ).unwrap();

        let htlc = Htlc::new(
            recipient_pubkey_hash,
            sender_pubkey_hash,
            SecretHash(secret_hash),
            900,
        );

        let address = htlc.compute_address(Network::BitcoinCoreRegtest);

        assert_eq!(
            address.to_string(),
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0"
        );
        // I did a bitcoin-rpc validateaddress
        // -> witness_program returned = sha256 of htlc script
        // Hence I guess it's correct!
    }
}
