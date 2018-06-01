extern crate bitcoin;
extern crate secp256k1;
#[macro_use]
extern crate lazy_static;

pub use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::util::address::Address;
use bitcoin::util::hash::Sha256dHash;
use bitcoin::util::privkey::Privkey as PrivateKey;
use secp256k1::Secp256k1;

use bitcoin::blockdata::transaction::TxIn;
use bitcoin::blockdata::transaction::TxOut;
use bitcoin::util::bip143::SighashComponents;
use secp256k1::Message;
use secp256k1::PublicKey;
use secp256k1::SecretKey;

//pub use bitcoin::network::serialize:: as serialize_transaction;

pub struct Secret(pub [u8; 32]);
pub struct Txid(pub [u8; 32]);

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

pub enum Error {
    SECPError(secp256k1::Error),
}

impl From<secp256k1::Error> for Error {
    fn from(error: secp256k1::Error) -> Self {
        Error::SECPError(error)
    }
}

fn generate_p2wsh_htlc_redeem_tx(
    txid: Txid,
    vout: u32,
    amount: u64,
    htlc_script: &Script,
    secret: &Secret,
    private_key: &PrivateKey,
    destination_addr: &Address,
) -> Result<Transaction, Error> {
    let public_key = PublicKey::from_secret_key(&*SECP, &private_key.secret_key())?;

    generate_segwit_redeem(
        Sha256dHash::from_data(&txid.0),
        vout,
        vec![
            Witness::Signature {
                private_key: private_key.secret_key(),
                prev_script: htlc_script,
            },
            Witness::Data(public_key.serialize().to_vec()),
            Witness::Data(secret.0.to_vec()),
            Witness::Data(vec![1 as u8]),
            Witness::Data(htlc_script.clone().into_vec()),
        ],
        destination_addr,
        amount,
    )
}

fn generate_segwit_redeem(
    txid: Sha256dHash,
    vout: u32,
    input_witness: Vec<Witness>,
    destination_address: &Address,
    amount: u64,
) -> Result<Transaction, Error> {
    let input = TxIn {
        prev_hash: txid,
        prev_index: vout,
        script_sig: Script::new(),
        sequence: 0xFFFFFFFF,
        witness: vec![],
    };

    let output = TxOut {
        value: amount,
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
                let to_sign = sighash_components.sighash_all(&input, &prev_script, amount);
                let to_sign = Message::from_slice(&to_sign[..])?;
                let signature = SECP.sign(&to_sign, &private_key)?;
                let mut binary_signature = signature.serialize_der(&*SECP).to_vec();
                binary_signature.push(1 as u8);
                witness_data.push(binary_signature);
            }
        }
    }
    //transaction.input[0].script_sig = Script::from(wrap_push_op(&vec![0xee as u8]));
    transaction.input[0].witness = witness_data;

    Ok(transaction)
}

extern crate bitcoin_rpc;
extern crate hex;

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin::util::privkey::Privkey;
    use bitcoin_rpc;
    use hex;
    use std::env::var;
    use std::str::FromStr;

    #[test]
    fn redeem_htlc() {
        let url = var("BITCOIN_RPC_URL").unwrap();
        let username = var("BITCOIN_RPC_USERNAME").unwrap();
        let password = var("BITCOIN_RPC_PASSWORD").unwrap();
        let private_key =
            Privkey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();

        let client =
            bitcoin_rpc::BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str());

        client.generate(432); //enable segwit
        let txid: String = client
            .send_to_address(
                bitcoin_rpc::Address::from(
                    "bcrt1qknf7h6xtuxa9s9yth5x8ejpxue745k85r7sxfz873nkqf7qr8d8qae4dce",
                ),
                1.0,
            )
            .unwrap()
            .into_result()
            .unwrap()
            .into();

        let alice_rpc_addr = client.get_new_address().unwrap().into_result().unwrap();

        let alice_addr = alice_rpc_addr.to_bitcoin_address().unwrap();

        let txid = hex::decode(txid).unwrap();
        //generate_p2wsh_htlc_redeem_tx()
    }
}
