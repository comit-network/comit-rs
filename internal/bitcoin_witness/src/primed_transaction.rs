use crate::witness::{UnlockParameters, Witness};
use bitcoin_support::{
    self, Address, Amount, Hash, OutPoint, Script, SigHashType, SighashComponents, Transaction,
    TxIn, TxOut,
};
use secp256k1_omni_context::{Message, SECP};

#[derive(Debug, PartialEq)]
pub enum Error {
    OverflowingFee,
    FeeHigherThanInputValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimedInput {
    input_parameters: UnlockParameters,
    value: Amount,
    previous_output: OutPoint,
}

impl PrimedInput {
    pub fn new(
        previous_output: OutPoint,
        value: Amount,
        input_parameters: UnlockParameters,
    ) -> PrimedInput {
        PrimedInput {
            input_parameters,
            value,
            previous_output,
        }
    }

    fn encode_witness_for_txin(&self, witness: &Witness) -> Vec<u8> {
        match witness {
            Witness::Data(data) => data.clone(),
            // We can't sign it yet so we put a placeholder
            // value of the most likely signature length
            Witness::Signature(_) => vec![0u8; 71],
            Witness::PublicKey(public_key) => public_key.serialize().to_vec(),
            Witness::Bool(_bool) => {
                if *_bool {
                    vec![1u8]
                } else {
                    vec![]
                }
            }
            Witness::PrevScript => self.input_parameters.prev_script.clone().into_bytes(),
        }
    }

    fn to_txin_without_signature(&self) -> TxIn {
        TxIn {
            previous_output: self.previous_output,
            script_sig: Script::new(),
            sequence: self.input_parameters.sequence,
            witness: self
                .input_parameters
                .witness
                .iter()
                .map(|witness| self.encode_witness_for_txin(witness))
                .collect(),
        }
    }
}

/// A transaction that's ready for signing
#[derive(Debug, Clone)]
pub struct PrimedTransaction {
    pub inputs: Vec<PrimedInput>,
    pub output_address: Address,
}

impl PrimedTransaction {
    fn _sign(self, transaction: &mut Transaction) {
        for (i, primed_input) in self.inputs.into_iter().enumerate() {
            let input_parameters = primed_input.input_parameters;
            for (j, witness) in input_parameters.witness.iter().enumerate() {
                if let Witness::Signature(secret_key) = witness {
                    let sighash_components = SighashComponents::new(transaction);
                    let hash_to_sign = sighash_components.sighash_all(
                        &transaction.input[i],
                        &input_parameters.prev_script,
                        primed_input.value.as_sat(),
                    );
                    // `from` should be used instead of `from_slice` once `ThirtyTwoByteHash` is
                    // implemented for Hashes See https://github.com/rust-bitcoin/rust-secp256k1/issues/106
                    let message_to_sign = Message::from_slice(&hash_to_sign.into_inner())
                        .expect("Should not fail because it is a hash");
                    let signature = SECP.sign(&message_to_sign, &secret_key);

                    let mut serialized_signature = signature.serialize_der().to_vec();
                    serialized_signature.push(SigHashType::All as u8);
                    transaction.input[i].witness[j] = serialized_signature;
                }
            }
        }
    }

    fn max_locktime(&self) -> Option<u32> {
        self.inputs
            .iter()
            .map(|input| input.input_parameters.locktime)
            .max()
    }

    pub fn sign_with_rate(self, fee_per_byte: usize) -> Result<Transaction, Error> {
        let mut transaction = self._transaction_without_signatures_or_output_values();

        let weight = transaction.get_weight();
        let fee = weight
            .checked_mul(fee_per_byte)
            .ok_or(Error::OverflowingFee)?;
        let fee = Amount::from_sat(fee as u64);

        if self.total_input_value() < fee {
            return Err(Error::FeeHigherThanInputValue);
        };

        transaction.output[0].value = (self.total_input_value() - fee).as_sat();

        transaction.lock_time = self.max_locktime().unwrap_or(0);

        self._sign(&mut transaction);
        Ok(transaction)
    }

    pub fn sign_with_fee(self, fee: Amount) -> Transaction {
        let mut transaction = self._transaction_without_signatures_or_output_values();

        transaction.output[0].value = (self.total_input_value() - fee).as_sat();

        transaction.lock_time = self.max_locktime().unwrap_or(0);

        self._sign(&mut transaction);
        transaction
    }

    pub fn total_input_value(&self) -> Amount {
        Amount::from_sat(
            self.inputs
                .iter()
                .fold(0, |acc, input| acc + input.value.as_sat()),
        )
    }

    fn _transaction_without_signatures_or_output_values(&self) -> Transaction {
        let output = TxOut {
            value: 0,
            script_pubkey: self.output_address.script_pubkey(),
        };

        Transaction {
            version: 2,
            lock_time: 0,
            input: self
                .inputs
                .iter()
                .map(PrimedInput::to_txin_without_signature)
                .collect(),
            output: vec![output],
        }
    }

    pub fn estimate_weight(&self) -> usize {
        self._transaction_without_signatures_or_output_values()
            .get_weight()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::p2wpkh::UnlockP2wpkh;
    use bitcoin_support::{Address, PrivateKey, Sha256dHash};
    use secp256k1_omni_context::KeyPair;
    use std::str::FromStr;

    #[test]
    fn estimate_weight_and_sign_with_fee_are_correct_p2wpkh() -> Result<(), failure::Error> {
        let private_key =
            PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm")?;
        let keypair: KeyPair = private_key.key.into();
        let dst_addr = Address::from_str("bc1q87v7fjxcs29xvtz8kdu79u2tjfn3ppu0c3e6cl")?;
        let txid = Sha256dHash::default();

        let primed_txn = PrimedTransaction {
            inputs: vec![PrimedInput::new(
                OutPoint {
                    txid,
                    vout: 1, // First number I found that gave me a 71 byte signature
                },
                Amount::from_btc(1.0).expect("Should convert 1.0 in bitcoin amount"),
                keypair.p2wpkh_unlock_parameters(),
            )],
            output_address: dst_addr,
        };
        let total_input_value = primed_txn.total_input_value();

        let rate = 42;

        let estimated_weight = primed_txn.estimate_weight();
        let transaction = primed_txn.sign_with_rate(rate).unwrap();

        let actual_weight = transaction.get_weight();
        let fee = total_input_value.as_sat() - transaction.output[0].value;

        assert_eq!(estimated_weight, actual_weight, "weight is correct");
        assert_eq!(fee, 18354, "actual fee paid is correct");
        Ok(())
    }
}
