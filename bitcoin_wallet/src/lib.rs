extern crate bitcoin;
extern crate secp256k1;
#[macro_use]
extern crate lazy_static;
extern crate common_types;
extern crate hex;

pub use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::util::address::Address;
use bitcoin::util::hash::Sha256dHash as Txid;
use bitcoin::util::privkey::Privkey as PrivateKey;
use secp256k1::Secp256k1;

use bitcoin::blockdata::transaction::TxIn;
use bitcoin::blockdata::transaction::TxOut;
use bitcoin::util::bip143::SighashComponents;
use bitcoin::util::hash::HexError;
use common_types::secret::Secret;
use secp256k1::Message;
use secp256k1::PublicKey;
use secp256k1::SecretKey;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

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
    BadHex(HexError),
}

impl From<secp256k1::Error> for Error {
    fn from(error: secp256k1::Error) -> Self {
        Error::SECPError(error)
    }
}

impl From<HexError> for Error {
    fn from(error: HexError) -> Self {
        Error::BadHex(error)
    }
}

fn generate_p2wsh_htlc_refund_tx(
    txid: &Txid,
    vout: u32,
    input_amount: u64,
    output_amount: u64,
    htlc_script: &Script,
    private_key: &PrivateKey,
    destination_addr: &Address,
) -> Result<Transaction, Error> {
    let public_key = PublicKey::from_secret_key(&*SECP, &private_key.secret_key())?;

    generate_segwit_redeem(
        txid,
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

fn generate_p2wsh_htlc_redeem_tx(
    txid: &Txid,
    vout: u32,
    input_amount: u64,
    output_amount: u64,
    htlc_script: &Script,
    secret: &Secret,
    private_key: &PrivateKey,
    destination_addr: &Address,
) -> Result<Transaction, Error> {
    let public_key = PublicKey::from_secret_key(&*SECP, &private_key.secret_key())?;

    generate_segwit_redeem(
        txid,
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

fn generate_segwit_redeem(
    txid: &Txid,
    vout: u32,
    input_amount: u64,
    output_amount: u64,
    input_witness: Vec<Witness>,
    destination_address: &Address,
) -> Result<Transaction, Error> {
    let input = TxIn {
        prev_hash: txid.clone(),
        prev_index: vout,
        script_sig: Script::new(),
        sequence: 0xFFFFFFFF,
        witness: vec![],
    };

    let output = TxOut {
        value: output_amount,
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
                    sighash_components.sighash_all(&input, &prev_script, input_amount);
                let message_to_sign = Message::from(hash_to_sign.data());
                let signature = SECP.sign(&message_to_sign, &private_key)?;
                let mut binary_signature = signature.serialize_der(&*SECP).to_vec();
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
    extern crate bitcoin_rpc;
    extern crate btc_htlc;

    use self::bitcoin_rpc::RpcError;
    use self::bitcoin_rpc::TxOutConfirmations;
    use self::btc_htlc::BtcHtlc;
    use super::*;
    use bitcoin::network::constants::Network;
    use bitcoin::network::serialize::serialize_hex;
    use bitcoin::util::privkey::Privkey;
    use std::env::var;
    use std::str::FromStr;

    fn create_client() -> bitcoin_rpc::BitcoinCoreClient {
        let url = var("BITCOIN_RPC_URL").unwrap();
        let username = var("BITCOIN_RPC_USERNAME").unwrap();
        let password = var("BITCOIN_RPC_PASSWORD").unwrap();

        let client =
            bitcoin_rpc::BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str());
        client.generate(432).unwrap(); //enable segwit
        client
    }

    fn private_key_to_address(privkey: &Privkey) -> Address {
        let secp = Secp256k1::new();
        let secret_pubkey = PublicKey::from_secret_key(&secp, privkey.secret_key()).unwrap();
        Address::p2wpkh(&secret_pubkey, Network::BitcoinCoreRegtest)
    }

    fn fund_htlc(
        client: &bitcoin_rpc::BitcoinCoreClient,
    ) -> (
        bitcoin_rpc::TransactionId,
        bitcoin_rpc::TransactionOutput,
        u64,
        Script,
        u32,
        Secret,
        PrivateKey,
        PrivateKey,
    ) {
        let success_privkey =
            Privkey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
        let success_address = private_key_to_address(&success_privkey);
        let refund_privkey =
            Privkey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
        let mut secret = Secret::from(*b"hello world, you are beautiful!!");
        let refund_address = private_key_to_address(&refund_privkey);
        let sequence_lock = 0;

        let amount = 100_000_001;

        let htlc = BtcHtlc::new(
            success_address,
            refund_address,
            secret.hash().clone(),
            sequence_lock,
            &Network::BitcoinCoreRegtest,
        ).unwrap();

        //        let htlc_address = bitcoin_rpc::Address::from(
        //            "bcrt1q8msll8hajpvw3ygt9gllx2pkpf0reuyps3x6xelrdk2uzyc77feqy84zm8",
        //        );
        //
        //        let htlc_script = Script::from(hex::decode(
        //            "63a82068d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec8876a9142e90d7ea212ad448ea0fa118c7975af9fca9a9956760b27576a914cef2b9c276e2553f86acffaea33a1cb66f1a8a8b6888ac"
        //        ).unwrap());

        let htlc_address = htlc.address();
        let rpc_htlc_address = bitcoin_rpc::Address::from(htlc_address.clone());
        let htlc_script = htlc.script();
        println!("{:?}", htlc_script);

        let txid = client
            .send_to_address(rpc_htlc_address.clone(), (amount as f64) / 100_000_000.0)
            .unwrap()
            .into_result()
            .unwrap();

        client.generate(1).unwrap();

        let _txn = client
            .get_transaction(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        println!("{:?}", _txn);

        let raw_htlc_txn = client
            .get_raw_transaction_serialized(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        println!("raw_htlc_txn: {:?}", raw_htlc_txn);

        let decoded_txn = client
            .decode_rawtransaction(raw_htlc_txn.clone())
            .unwrap()
            .into_result()
            .unwrap();

        let vout = decoded_txn
            .vout
            .iter()
            .find(|txout| txout.matches_address(&rpc_htlc_address))
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

    #[test]
    fn redeem_htlc() {
        let client = create_client();

        let (txid, vout, input_amount, htlc_script, _, secret, private_key, _) = fund_htlc(&client);

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();
        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let txid_hex: String = txid.into();
        let txid = Txid::from_hex(txid_hex.as_str()).unwrap();

        let fee = 1000;

        let redeem_tx = generate_p2wsh_htlc_redeem_tx(
            &txid,
            vout.n,
            input_amount,
            input_amount - fee,
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

        let unspent = client
            .list_unspent(
                TxOutConfirmations::AtLeast(1),
                None,
                Some(vec![alice_rpc_addr]),
            )
            .unwrap()
            .into_result()
            .unwrap();

        assert!(
            unspent
                .iter()
                .find(|utxo| utxo.txid == rpc_redeem_txid)
                .is_some(),
            "utxo should exist after redeeming htlc"
        );
    }

    #[test]
    fn redeem_refund_htlc() {
        //        address2=bcrt1qemetnsnkuf2nlp4vl7h2xwsukeh34z5tugwqc8
        //        privateKey2=cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx
        //        publicKey2=02e7ab81f151bd057ad30827653ede1734e80f4145b12a2b89038d42604d8c4d86
        //        hashedPublicKey=$(bx bitcoin160 $publicKey2)
        //        =cef2b9c276e2553f86acffaea33a1cb66f1a8a8b

        let client = create_client();

        let (txid, vout, input_amount, htlc_script, csv_arg, secret, _, private_key) =
            fund_htlc(&client);

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();
        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let txid_hex: String = txid.clone().into();
        let txid_sha256d = Txid::from_hex(txid_hex.as_str()).unwrap();

        let fee = 1000;

        let redeem_tx = generate_p2wsh_htlc_refund_tx(
            &txid_sha256d,
            vout.n,
            input_amount,
            input_amount - fee,
            &htlc_script,
            &private_key,
            &alice_addr,
        ).unwrap();

        println!("raw_redeem_txn: {:?}", serialize_hex(&redeem_tx).unwrap());

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
        assert!(error.message.contains("Locktime"));

        client.generate(91).unwrap();

        let _txn = client
            .get_transaction(&txid)
            .unwrap()
            .into_result()
            .unwrap();
        println!("{:?}", _txn);

        let rpc_redeem_txid = client
            .send_raw_transaction(raw_redeem_tx)
            .unwrap()
            .into_result()
            .unwrap();
    }
}
