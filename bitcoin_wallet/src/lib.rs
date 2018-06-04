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

    use self::bitcoin_rpc::TxOutConfirmations;
    use super::*;
    use bitcoin::network::serialize::serialize_hex;
    use bitcoin::util::privkey::Privkey;
    use std::env::var;
    use std::str::FromStr;

    #[test]
    fn redeem_htlc() {
        let url = var("BITCOIN_RPC_URL").unwrap();
        let username = var("BITCOIN_RPC_USERNAME").unwrap();
        let password = var("BITCOIN_RPC_PASSWORD").unwrap();
        let input_amount = 100_000_001;

        let private_key =
            Privkey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();

        let htlc_address = bitcoin_rpc::Address::from(
            "bcrt1q8msll8hajpvw3ygt9gllx2pkpf0reuyps3x6xelrdk2uzyc77feqy84zm8",
        );

        let htlc_script = Script::from(hex::decode(
                "63a82068d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec8876a9142e90d7ea212ad448ea0fa118c7975af9fca9a9956760b27576a914cef2b9c276e2553f86acffaea33a1cb66f1a8a8b6888ac"
        ).unwrap());

        let client =
            bitcoin_rpc::BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str());

        client.generate(432).unwrap(); //enable segwit

        let txid = client
            .send_to_address(htlc_address.clone(), (input_amount as f64) / 100_000_000.0)
            .unwrap()
            .into_result()
            .unwrap();

        client.generate(1).unwrap();

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

        let tx_out = decoded_txn
            .vout
            .iter()
            .find(|txout| txout.matches_address(&htlc_address))
            .unwrap();

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();
        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let txid_hex: String = txid.into();
        let txid = Txid::from_hex(txid_hex.as_str()).unwrap();

        let fee = 1000;

        let redeem_tx = generate_p2wsh_htlc_redeem_tx(
            &txid,
            tx_out.n,
            input_amount,
            input_amount - fee,
            &htlc_script,
            &Secret::from(*b"hello world, you are beautiful!!"),
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
}
