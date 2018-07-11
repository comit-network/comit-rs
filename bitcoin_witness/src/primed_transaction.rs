use bitcoin_support::{Address, BitcoinQuantity, Script, Sha256dHash, SighashComponents,
                      Transaction, TxIn, TxOut, Weight};
use secp256k1_support::{DerSerializableSignature, Message, SignMessage};
use witness::{Witness, WitnessMethod};

pub struct PrimedInput {
    pub sequence: u32,
    pub witness: Vec<Witness>,
    pub value: BitcoinQuantity,
    pub vout: u32,
    pub txid: Sha256dHash,
    pub prev_script: Script,
}

impl PrimedInput {
    pub fn new<W: WitnessMethod>(
        txid: Sha256dHash,
        vout: u32,
        value: BitcoinQuantity,
        witness_method: W,
    ) -> PrimedInput {
        PrimedInput {
            sequence: witness_method.sequence(),
            prev_script: witness_method.prev_script(),
            witness: witness_method.into_witness(),
            value,
            vout,
            txid,
        }
    }

    fn to_txin(&self) -> TxIn {
        TxIn {
            prev_hash: self.txid,
            prev_index: self.vout,
            script_sig: Script::new(),
            sequence: self.sequence,
            // We can't calculate actual witness data yet but we need
            // to put things that are (roughly) the same size in there
            // so we can calculate the (roughly) correct weight prior
            // to signing
            witness: self.witness
                .iter()
                .map(|witness| match witness {
                    Witness::Data(data) => vec![0; data.len()],
                    Witness::Signature(_) => vec![0; 71],
                    Witness::PublicKey(_) => vec![0; 33],
                    Witness::Bool(_bool) => if *_bool {
                        vec![1]
                    } else {
                        vec![]
                    },
                    Witness::PrevScript => vec![0; self.prev_script.len()],
                })
                .collect(),
        }
    }
}

/// A transaction that's ready for signing
pub struct PrimedTransaction {
    pub inputs: Vec<PrimedInput>,
    pub output_address: Address,
    pub locktime: u32,
}

impl PrimedTransaction {
    fn _encode_witness(
        primed_input: PrimedInput,
        transaction: &Transaction,
        index: usize,
    ) -> Vec<Vec<u8>> {
        let (witnesses, value, prev_script) = (
            primed_input.witness,
            primed_input.value,
            primed_input.prev_script,
        );

        witnesses
            .into_iter()
            .map(|witness| {
                match witness {
                    Witness::Bool(_bool) => if _bool {
                        vec![1u8]
                    } else {
                        vec![]
                    },
                    Witness::PrevScript => prev_script.clone().into_vec(),
                    Witness::Data(data) => data,
                    Witness::Signature(secret_key) => {
                        let sighash_components = SighashComponents::new(transaction);
                        let hash_to_sign = sighash_components.sighash_all(
                            &transaction.input[index],
                            &prev_script,
                            value.satoshi(),
                        );
                        let message_to_sign = Message::from(hash_to_sign.data());

                        // TODO: remove unwrap once we have
                        // incorporated Thomas' improvements to SECP
                        let signature = secret_key.sign_ecdsa(message_to_sign);

                        let mut serialized_signature = signature.serialize_signature_der();
                        // Without this 1 at the end you get "Non-canonical DER Signature"
                        serialized_signature.push(1u8);
                        serialized_signature
                    }
                    Witness::PublicKey(public_key) => public_key.serialize().to_vec(),
                }
            })
            .collect()
    }

    fn _sign(self, transaction: &mut Transaction) {
        let witness_by_input: Vec<Vec<Vec<u8>>> = self.inputs
            .into_iter()
            .enumerate()
            .map(|(i, primed_input)| Self::_encode_witness(primed_input, &transaction, i))
            .collect();

        for (i, witness) in witness_by_input.into_iter().enumerate() {
            transaction.input[i].witness = witness;
        }
    }

    pub fn sign_with_rate(self, fee_per_byte: f64) -> Transaction {
        let mut transaction = self._dummy_transaction();

        let weight: Weight = transaction.get_weight().into();
        let fee = weight.calculate_fee(fee_per_byte);

        transaction.output[0].value = (self.total_input_value() - fee).satoshi();

        self._sign(&mut transaction);
        transaction
    }

    pub fn sign_with_fee(self, fee: BitcoinQuantity) -> Transaction {
        let mut transaction = self._dummy_transaction();

        transaction.output[0].value = (self.total_input_value() - fee).satoshi();

        self._sign(&mut transaction);
        transaction
    }

    pub fn total_input_value(&self) -> BitcoinQuantity {
        BitcoinQuantity::from_satoshi(
            self.inputs
                .iter()
                .fold(0, |acc, input| acc + input.value.satoshi()),
        )
    }

    fn _dummy_transaction(&self) -> Transaction {
        let output = TxOut {
            value: 0,
            script_pubkey: self.output_address.script_pubkey(),
        };

        Transaction {
            version: 2,
            lock_time: self.locktime,
            input: self.inputs.iter().map(PrimedInput::to_txin).collect(),
            output: vec![output],
        }
    }

    pub fn estimate_weight(&self) -> Weight {
        self._dummy_transaction().get_weight().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::{Address, PrivateKey, Sha256dHash};
    use std::str::FromStr;
    use witness::WitnessP2pkh;

    #[test]
    fn estimate_weight_and_sign_with_fee_are_correct_p2wpkh() {
        let private_key =
            PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
        let dst_addr = Address::from_str("bc1q87v7fjxcs29xvtz8kdu79u2tjfn3ppu0c3e6cl").unwrap();
        let txid = Sha256dHash::default();

        let primed_txn = PrimedTransaction {
            inputs: vec![
                PrimedInput::new(
                    txid,
                    1, // First number I found that gave me a 71 byte signature
                    BitcoinQuantity::from_bitcoin(1.0),
                    WitnessP2pkh(private_key.secret_key().clone()),
                ),
            ],
            output_address: dst_addr,
            locktime: 0,
        };
        let total_input_value = primed_txn.total_input_value();

        let rate = 42.0;

        let estimated_weight = primed_txn.estimate_weight();
        let transaction = primed_txn.sign_with_rate(rate);

        let actual_weight: Weight = transaction.get_weight().into();
        let fee = total_input_value.satoshi() - transaction.output[0].value;

        assert_eq!(estimated_weight, actual_weight, "weight is correct");
        assert_eq!(fee, 4589, "actual fee paid is correct");
    }
}
