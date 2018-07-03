use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::blockdata::transaction::TxIn;
use bitcoin::blockdata::transaction::TxOut;
use bitcoin::util::address::Address;
use bitcoin::util::bip143::SighashComponents;
use bitcoin_rpc;
use bitcoin_rpc::TransactionId;
use common_types::BitcoinQuantity;
use common_types::secret::Secret;
use key::PrivateKey;
use secp256k1;
use secp256k1::Message;
use secp256k1::PublicKey;
use secp256k1::SecretKey;
use std::fmt;
use std::str::FromStr;
use weight::Weight;

enum Witness<'a> {
    Data(Vec<u8>),
    Signature {
        private_key: &'a SecretKey,
        prev_script: &'a Script,
    },
}

#[derive(Debug)]
pub enum Error {
    SECPError(secp256k1::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SECPError(error) => {
                write!(f, "Something weird happened with SECP256k1 - {}", error)
            }
        }
    }
}

impl From<secp256k1::Error> for Error {
    fn from(error: secp256k1::Error) -> Self {
        Error::SECPError(error)
    }
}

pub fn generate_p2wsh_htlc_refund_tx(
    txid: &bitcoin_rpc::TransactionId,
    vout: u32,
    nsequence: u32,
    input_amount: BitcoinQuantity,
    output_amount: BitcoinQuantity,
    htlc_script: &Script,
    private_key: &PrivateKey,
    destination_addr: &Address,
) -> Result<Transaction, Error> {
    let public_key = PublicKey::from_secret_key(&*super::SECP, &private_key.secret_key())?;

    generate_segwit_redeem(
        txid,
        nsequence,
        vout,
        input_amount,
        output_amount,
        vec![
            Witness::Signature {
                private_key: private_key.secret_key(),
                prev_script: htlc_script,
            },
            Witness::Data(public_key.serialize().to_vec()),
            Witness::Data(vec![]),
            Witness::Data(htlc_script.clone().into_vec()),
        ],
        destination_addr,
    )
}

pub fn generate_p2wsh_htlc_redeem_tx(
    txid: &TransactionId,
    vout: u32,
    input_amount: BitcoinQuantity,
    output_amount: BitcoinQuantity,
    htlc_script: &Script,
    secret: &Secret,
    private_key: &PrivateKey,
    destination_addr: &Address,
) -> Result<Transaction, Error> {
    let public_key = PublicKey::from_secret_key(&*super::SECP, &private_key.secret_key())?;

    generate_segwit_redeem(
        txid,
        0xFFFFFFFF,
        vout,
        input_amount,
        output_amount,
        vec![
            Witness::Signature {
                private_key: private_key.secret_key(),
                prev_script: htlc_script,
            },
            Witness::Data(public_key.serialize().to_vec()),
            Witness::Data(secret.raw_secret().to_vec()),
            Witness::Data(vec![1 as u8]),
            Witness::Data(htlc_script.clone().into_vec()),
        ],
        destination_addr,
    )
}

pub fn estimate_weight_of_redeem_tx_with_script(script: &Script) -> Weight {
    let dummy_tx_id = TransactionId::from_str(
        "0000000000000000000000000000000000000000000000000000000000000000",
    ).unwrap();
    let dummy_secret = Secret::from_str(
        "0000000000000000000000000000000000000000000000000000000000000000",
    ).unwrap();
    let dummy_private_key =
        PrivateKey::from_str("cVt4o7BGAig1UXywgGSmARhxMdzP5qvQsxKkSsc1XEkw3tDTQFpy").unwrap();
    let dummy_destination_address =
        Address::from_str("33iFwdLuRpW1uK1RTRqsoi8rR4NpDzk66k").unwrap();

    let transaction = generate_p2wsh_htlc_redeem_tx(
        &dummy_tx_id,
        0,
        BitcoinQuantity::from_bitcoin(1.0),
        BitcoinQuantity::from_bitcoin(1.0),
        script,
        &dummy_secret,
        &dummy_private_key,
        &dummy_destination_address,
    );

    Weight::from(transaction.unwrap().get_weight())
}

fn generate_segwit_redeem(
    txid: &TransactionId,
    nsequence: u32,
    vout: u32,
    input_amount: BitcoinQuantity,
    output_amount: BitcoinQuantity,
    input_witness: Vec<Witness>,
    destination_address: &Address,
) -> Result<Transaction, Error> {
    let input = TxIn {
        prev_hash: txid.clone().into(),
        prev_index: vout,
        script_sig: Script::new(),
        sequence: nsequence,
        witness: vec![],
    };

    let output = TxOut {
        value: output_amount.satoshi(),
        script_pubkey: destination_address.script_pubkey(),
    };

    let mut transaction = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![input.clone()],
        output: vec![output],
    };

    let mut witness_data: Vec<Vec<u8>> = vec![];

    for witness in input_witness {
        match witness {
            Witness::Data(data) => witness_data.push(data),
            Witness::Signature {
                private_key,
                prev_script,
            } => {
                let sighash_components = SighashComponents::new(&transaction);
                let hash_to_sign =
                    sighash_components.sighash_all(&input, &prev_script, input_amount.satoshi());
                let message_to_sign = Message::from(hash_to_sign.data());
                let signature = super::SECP.sign(&message_to_sign, &private_key)?;
                let mut binary_signature = signature.serialize_der(&*super::SECP).to_vec();
                binary_signature.push(1 as u8);
                witness_data.push(binary_signature);
            }
        }
    }
    transaction.input[0].witness = witness_data;

    Ok(transaction)
}

#[cfg(test)]
mod tests {
    extern crate bitcoin_htlc;
    extern crate bitcoin_types;

    use self::bitcoin_htlc::Htlc;
    use self::bitcoin_rpc::TransactionId;
    use self::bitcoin_rpc::TxOutConfirmations;
    use self::bitcoin_types::PubkeyHash;
    use super::*;
    use bitcoin::network::constants::Network;
    use bitcoin::network::serialize::serialize_hex;
    use bitcoin::util::hash::Hash160;
    use bitcoin::util::privkey::Privkey;
    use bitcoin_rpc::BitcoinRpcApi;
    use bitcoin_rpc::test_support::BitcoinNode;
    use std::str::FromStr;

    extern crate hex;

    fn private_key_to_pubkey_hash(privkey: &Privkey) -> PubkeyHash {
        let secret_pubkey =
            PublicKey::from_secret_key(&*super::super::SECP, privkey.secret_key()).unwrap();
        let pubkey_serialized = secret_pubkey.serialize();
        let hash160 = Hash160::from_data(&pubkey_serialized);
        let pubkey_hash = PubkeyHash::from(hash160);
        pubkey_hash
    }

    fn fund_htlc(
        client: &bitcoin_rpc::BitcoinCoreClient,
    ) -> (
        bitcoin_rpc::TransactionId,
        bitcoin_rpc::TransactionOutput,
        BitcoinQuantity,
        Script,
        u32,
        Secret,
        PrivateKey,
        PrivateKey,
    ) {
        let success_privkey =
            Privkey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
        let success_pubkey_hash = private_key_to_pubkey_hash(&success_privkey);
        let refund_privkey =
            Privkey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
        let secret = Secret::from(*b"hello world, you are beautiful!!");
        let refund_pubkey_hash = private_key_to_pubkey_hash(&refund_privkey);
        let sequence_lock = 10;

        let amount = BitcoinQuantity::from_satoshi(100_000_001);

        let htlc = Htlc::new(
            success_pubkey_hash,
            refund_pubkey_hash,
            secret.hash(),
            sequence_lock,
        );

        let htlc_address = htlc.compute_address(Network::BitcoinCoreRegtest);
        let rpc_htlc_address = bitcoin_rpc::Address::from(htlc_address.clone());
        let htlc_script = htlc.script();

        let txid = client
            .send_to_address(&rpc_htlc_address, amount.bitcoin())
            .unwrap()
            .into_result()
            .unwrap();

        client.generate(1).unwrap();

        let _txn = client
            .get_transaction(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        let raw_htlc_txn = client
            .get_raw_transaction_serialized(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        let decoded_txn = client
            .decode_rawtransaction(raw_htlc_txn.clone())
            .unwrap()
            .into_result()
            .unwrap();

        let vout = decoded_txn
            .vout
            .iter()
            .find(|txout| {
                hex::decode(&txout.script_pub_key.hex).unwrap()
                    == htlc_address.script_pubkey().into_vec()
            })
            .unwrap();

        (
            txid,
            vout.clone(),
            amount,
            htlc_script.clone(),
            sequence_lock,
            secret,
            success_privkey,
            refund_privkey,
        )
    }

    fn check_utxo_at_address(
        client: &bitcoin_rpc::BitcoinCoreClient,
        address: &bitcoin_rpc::Address,
        txid: &TransactionId,
    ) -> bool {
        let unspent = client
            .list_unspent(
                TxOutConfirmations::AtLeast(1),
                None,
                Some(vec![address.clone()]),
            )
            .unwrap()
            .into_result()
            .unwrap();

        unspent.iter().find(|utxo| utxo.txid == *txid).is_some()
    }

    #[test]
    fn redeem_htlc() {
        let bitcoin_node = BitcoinNode::new();
        let client = bitcoin_node.get_client();
        let _ = client.generate(432);

        let (txid, vout, input_amount, htlc_script, _, secret, private_key, _) = fund_htlc(&client);

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();
        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let fee = BitcoinQuantity::from_satoshi(1000);
        let output_amount = input_amount.clone() - fee;

        let redeem_tx = generate_p2wsh_htlc_redeem_tx(
            &txid,
            vout.n,
            input_amount,
            output_amount,
            &htlc_script,
            &secret,
            &private_key,
            &alice_addr,
        ).unwrap();

        let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

        let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

        let rpc_redeem_txid = client
            .send_raw_transaction(raw_redeem_tx)
            .unwrap()
            .into_result()
            .unwrap();

        client.generate(1).unwrap();

        assert!(
            check_utxo_at_address(&client, &alice_rpc_addr, &rpc_redeem_txid),
            "utxo should exist after redeeming htlc"
        );
    }

    #[test]
    fn redeem_refund_htlc() {
        let bitcoin_node = BitcoinNode::new();
        let client = bitcoin_node.get_client();
        let _ = client.generate(432);

        let (txid, vout, input_amount, htlc_script, nsequence, _, _, private_key) =
            fund_htlc(&client);

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();
        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let fee = BitcoinQuantity::from_satoshi(1000);
        let output_amount = input_amount.clone() - fee;

        let redeem_tx = generate_p2wsh_htlc_refund_tx(
            &txid,
            vout.n,
            nsequence,
            input_amount,
            output_amount,
            &htlc_script,
            &private_key,
            &alice_addr,
        ).unwrap();

        let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

        let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

        let rpc_redeem_txid_error = client
            .send_raw_transaction(raw_redeem_tx.clone())
            .unwrap()
            .into_result();

        assert!(rpc_redeem_txid_error.is_err());
        let error = rpc_redeem_txid_error.unwrap_err();

        assert_eq!(error.code, -26);
        ///RPC_VERIFY_REJECTED = -26, !< Transaction or block was rejected by network rules
        assert!(error.message.contains("non-BIP68-final"));

        client.generate(nsequence).unwrap();

        let _txn = client
            .get_transaction(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        let rpc_redeem_txid = client
            .send_raw_transaction(raw_redeem_tx)
            .unwrap()
            .into_result()
            .unwrap();

        client.generate(1).unwrap();

        assert!(
            check_utxo_at_address(&client, &alice_rpc_addr, &rpc_redeem_txid),
            "utxo should exist after refunding htlc"
        );
    }

    #[test]
    fn given_a_script_should_estimate_weight_of_transaction() {
        let script = Script::default();

        let weight = estimate_weight_of_redeem_tx_with_script(&script);

        assert_eq!(weight, Weight::from(477));
    }
}
