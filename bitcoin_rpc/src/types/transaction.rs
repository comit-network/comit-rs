use bitcoin;
use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::Transaction as BitcoinTransaction;
use bitcoin::network::serialize as bitcoin_serialize;
use bitcoin::util::hash::{HexError, Sha256dHash};
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use types::*;

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionId(Sha256dHash);

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(self.0.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for TransactionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = TransactionId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("a hex encoded 32 byte value")
            }

            fn visit_str<E>(self, v: &str) -> Result<TransactionId, E>
            where
                E: de::Error,
            {
                TransactionId::from_str(v).map_err(|hexerr| E::custom(format!("{}", hexerr)))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for TransactionId {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.be_hex_string().as_str())
    }
}

impl From<TransactionId> for Sha256dHash {
    fn from(txid: TransactionId) -> Self {
        txid.0
    }
}

impl FromStr for TransactionId {
    type Err = HexError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Sha256dHash::from_hex(s).map(TransactionId)
    }
}

#[derive(Debug, PartialEq, Clone)]
//TODO: can be used once https://github.com/rust-bitcoin/rust-bitcoin/issues/104 is fixed
pub struct TransactionWrapper(BitcoinTransaction);

impl Serialize for TransactionWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let hex = bitcoin_serialize::serialize_hex(&self.0)
            .map_err(|err| Error::custom(format!("{}", err)))?;
        serializer.serialize_str(hex.as_str())
    }
}

impl<'de> Deserialize<'de> for TransactionWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = TransactionWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("A raw transaction in hex")
            }

            fn visit_str<E>(self, v: &str) -> Result<TransactionWrapper, E>
            where
                E: de::Error,
            {
                let tx =
                    TransactionWrapper::from_str(v).map_err(|err| E::custom(format!("{}", err)))?;
                Ok(tx)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl From<TransactionWrapper> for BitcoinTransaction {
    fn from(tx_wrapper: TransactionWrapper) -> Self {
        tx_wrapper.0
    }
}

impl From<BitcoinTransaction> for TransactionWrapper {
    fn from(tx: BitcoinTransaction) -> Self {
        TransactionWrapper(tx)
    }
}

impl FromStr for TransactionWrapper {
    type Err = bitcoin::util::Error;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let hex_tx = bitcoin::util::misc::hex_bytes(s)?;
        let tx: BitcoinTransaction = bitcoin_serialize::deserialize(&hex_tx)?;
        Ok(TransactionWrapper(tx))
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SerializedRawTransaction(String);

from_str!(SerializedRawTransaction);

impl From<SerializedRawTransaction> for BitcoinTransaction {
    fn from(serialized_tx: SerializedRawTransaction) -> Self {
        //TODO: expect can be removed once https://github.com/rust-bitcoin/rust-bitcoin/issues/104 is fixed
        let wrapper = TransactionWrapper::from_str(serialized_tx.0.as_str()).expect(
            "Conversion to bitcoin[..]::Transaction failed, does your transaction have inputs?",
        );
        let bitcoin_tx: BitcoinTransaction = wrapper.into();
        bitcoin_tx
    }
}

impl From<BitcoinTransaction> for SerializedRawTransaction {
    fn from(tx: BitcoinTransaction) -> Self {
        //TODO: expect can be removed once https://github.com/rust-bitcoin/rust-bitcoin/issues/104 is fixed
        SerializedRawTransaction(
            bitcoin_serialize::serialize_hex(&tx)
                .expect("Conversion from bitcoin[..]::Transaction to hex failed"),
        )
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Transaction {
    amount: f64,
    fee: Option<f64>,
    confirmations: u32,
    generated: Option<bool>,
    blockhash: Option<BlockHash>,
    /// Unix Timestamp
    blocktime: Option<u64>,
    /// Unix Timestamp
    blockindex: Option<u64>,
    walletconflicts: Vec<TransactionId>,
    txid: TransactionId,
    /// Unix Timestamp
    time: u64,
    /// Unix Timestamp
    timereceived: u64,
    comment: Option<String>,
    to: Option<String>,
    #[serde(rename = "bip125-replaceable")]
    /// yes|no|unknown: TODO: Create enum if needed
    bip125_replaceable: String,
    details: Vec<Detail>,
    hex: SerializedRawTransaction,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Detail {
    account: String,
    address: Option<Address>,
    /// send|receive|immature|generate|orphan TODO: Create enum if needed
    category: String,
    amount: f64,
    fee: Option<f64>,
    vout: u32,
    #[serde(rename = "involvesWatchonly")]
    involves_watchonly: Option<bool>,
    abandoned: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct DecodedRawTransaction {
    pub txid: TransactionId,
    pub hash: String,
    pub size: u32,
    pub vsize: u32,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<TransactionInput>,
    pub vout: Vec<TransactionOutput>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct VerboseRawTransaction {
    txid: TransactionId,
    hash: String,
    size: u32,
    vsize: u32,
    version: u32,
    locktime: u32,
    vin: Vec<TransactionInput>,
    vout: Vec<TransactionOutput>,
    hex: SerializedRawTransaction,
    blockhash: BlockHash,
    confirmations: i32,
    time: u64,
    blocktime: u64,
}

// TODO: Create serializer and deserializer that can create this struct from the only the hex string
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ScriptSig {
    asm: String,
    hex: String,
}

/// Transaction input can either be a regular transaction or a coinbase transaction.
/// The have different fields, but most of the time, we will be interactings with regular transactions.
/// For deserialization compatiblity, we define all the fields as Option<T> and provide accessors.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TransactionInput {
    txid: Option<TransactionId>,
    vout: Option<u32>,
    #[serde(rename = "scriptSig")]
    script_sig: Option<ScriptSig>,

    coinbase: Option<String>,

    sequence: u64,
}

impl TransactionInput {
    pub fn txid(&self) -> &TransactionId {
        self.txid.as_ref().expect("This is a coinbase transaction.")
    }

    pub fn vout(&self) -> u32 {
        self.vout.expect("This is a coinbase transaction.")
    }

    pub fn script_sig(&self) -> &ScriptSig {
        self.script_sig
            .as_ref()
            .expect("This is a coinbase transaction.")
    }

    pub fn coinbase(&self) -> &str {
        self.coinbase
            .as_ref()
            .expect("This is NOT a coinbase transaction.")
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct TransactionOutput {
    pub value: f64,
    pub n: u32,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: ScriptPubKey,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct UnspentTransactionOutput {
    pub txid: TransactionId,
    pub vout: u32,
    pub address: Option<Address>,
    pub account: Option<String>,
    #[serde(rename = "scriptPubKey")]
    #[serde(with = "script_serde")]
    pub script_pub_key: Script,
    pub redeem_script: Option<String>,
    pub amount: f64,
    pub confirmations: i32,
    pub spendable: bool,
    pub solvable: bool,
    pub safe: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct NewTransactionInput {
    txid: TransactionId,
    vout: u32,
    sequence: Option<u32>,
}

impl NewTransactionInput {
    pub fn from_utxo(utxo: &UnspentTransactionOutput) -> Self {
        NewTransactionInput {
            txid: utxo.txid.clone(),
            vout: utxo.vout,
            sequence: None,
        }
    }
}

pub type NewTransactionOutput = HashMap<Address, f64>;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TransactionOutputDetail {
    txid: TransactionId,
    vout: u32,
    #[serde(rename = "scriptPubKey", with = "script_serde")]
    script_pub_key: Script,
    #[serde(rename = "redeemScript")]
    redeem_script: Option<RedeemScript>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SigningError {
    txid: TransactionId,
    vout: u32,
    // TODO: Use ScriptSig type here once we have the (de)serializer
    #[serde(rename = "scriptSig")]
    script_sig: String,
    sequence: u32,
    error: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SigningResult {
    hex: String,
    complete: bool,
    errors: Option<Vec<SigningError>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct FundingOptions {
    #[serde(rename = "changeAddress", skip_serializing_if = "Option::is_none")]
    change_address: Option<Address>,
    #[serde(rename = "changePosition", skip_serializing_if = "Option::is_none")]
    change_position: Option<u32>,
    #[serde(rename = "includeWatching", skip_serializing_if = "Option::is_none")]
    include_watching: Option<bool>,
    #[serde(rename = "lockUnspents", skip_serializing_if = "Option::is_none")]
    lock_unspents: Option<bool>,
    #[serde(rename = "reserveChangeKey", skip_serializing_if = "Option::is_none")]
    reserve_change_key: Option<bool>,
    #[serde(rename = "feeRate", skip_serializing_if = "Option::is_none")]
    fee_rate: Option<u32>,
    #[serde(rename = "subtractFeeFromOutputs", skip_serializing_if = "Option::is_none")]
    subtract_fee_from_outputs: Option<Vec<u32>>,
}

impl FundingOptions {
    pub fn new() -> Self {
        FundingOptions {
            change_address: None,
            change_position: None,
            include_watching: None,
            lock_unspents: None,
            reserve_change_key: None,
            fee_rate: None,
            subtract_fee_from_outputs: None,
        }
    }

    pub fn with_change_address(self, address: &Address) -> Self {
        FundingOptions {
            change_address: Some(address.clone()),
            ..self
        }
    }

    // TODO: Implement other builder methods
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct FundingResult {
    hex: SerializedRawTransaction,
    fee: f64,
    // TODO: This is -1 if no change output was added. Add custom deserializer that converts to Option<u32>
    #[serde(rename = "changepos")]
    change_pos: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::blockdata::script::Script;
    use serde_json;
    use std_hex;

    #[test]
    fn should_deserialize_transaction() {
        let tx = r#"{"amount":0.00000000,"confirmations":2,"generated":true,"blockhash":"33ba1550e92f5c73fa852c93d3f32a2ba0931cf64bc38b9be063a5b0f8d07440","blockindex":0,"blocktime":1525055404,"txid":"7e7c52b1f46e7ea2511e885d8c0e5df9297f65b6fff6907ceb1377d0582e45f4","walletconflicts":[],"time":1525055404,"timereceived":1525055404,"bip125-replaceable":"no","details":[{"account":"","address":"n3e8z6HmMDPQGDr3seFjpg88PeagBg2EeR","category":"immature","amount":50.00000000,"vout":0}],"hex":"020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401180101ffffffff0200f2052a01000000232102ec5601272cb71c84d0216661534cfea0d617decbc84a626b7f9f30fb4b0e65d9ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000"}"#;

        let _tx: Transaction = serde_json::from_str(tx).unwrap();
    }

    #[test]
    fn should_deserialize_decoded_raw_transaction() {
        let json = r#"
        {
            "txid": "52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46",
            "hash": "52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46",
            "size": 223,
            "vsize": 223,
            "version": 1,
            "locktime": 0,
            "vin": [
                {
                    "txid": "2ac0daff49a4ff82a35a4864797f99f23c396b0529c5ba1e04b3d7b97521feba",
                    "vout": 0,
                    "scriptSig": {
                        "asm": "3044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b[ALL] 0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8",
                        "hex": "473044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b01410479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8"
                    },
                    "sequence": 4294967295
                }
            ],
            "vout": [
                {
                    "value": 0.06990000,
                    "n": 0,
                    "scriptPubKey": {
                    "asm": "OP_DUP OP_HASH160 01b81d5fa1e55e069e3cc2db9c19e2e80358f306 OP_EQUALVERIFY OP_CHECKSIG",
                    "hex": "76a91401b81d5fa1e55e069e3cc2db9c19e2e80358f30688ac",
                    "reqSigs": 1,
                    "type": "pubkeyhash",
                    "addresses": [
                        "1A6Ei5cRfDJ8jjhwxfzLJph8B9ZEthR9Z"
                    ]
                    }
                }
            ]
        }
        "#;

        let tx: DecodedRawTransaction = serde_json::from_str(json).unwrap();

        assert_eq!(tx, DecodedRawTransaction {
            txid: TransactionId::from_str("52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46").unwrap(),
            hash: "52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46".to_string(),
            size: 223,
            vsize: 223,
            version: 1,
            locktime: 0,
            vin: vec![
                TransactionInput {
                    txid: Some(TransactionId::from_str("2ac0daff49a4ff82a35a4864797f99f23c396b0529c5ba1e04b3d7b97521feba").unwrap()),
                    vout: Some(0),
                    script_sig: Some(ScriptSig {
                        asm: "3044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b[ALL] 0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8".to_string(),
                        hex: "473044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b01410479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8".to_string(),
                    }),
                    coinbase: None,
                    sequence: 4294967295,
                }
            ],
            vout: vec![
                TransactionOutput {
                    value: 0.06990000,
                    n: 0,
                    script_pub_key: ScriptPubKey {
                        asm: "OP_DUP OP_HASH160 01b81d5fa1e55e069e3cc2db9c19e2e80358f306 OP_EQUALVERIFY OP_CHECKSIG".to_string(),
                        hex: Script::from(std_hex::decode("76a91401b81d5fa1e55e069e3cc2db9c19e2e80358f30688ac").unwrap()),
                        req_sigs: Some(1),
                        script_type: ScriptType::PubKeyHash,
                        addresses: Some(vec![
                            Address::from_str("1A6Ei5cRfDJ8jjhwxfzLJph8B9ZEthR9Z").unwrap()
                        ]),
                    },
                }
            ],
        })
    }

    #[test]
    fn should_deserialize_serialized_raw_transaction() {
        let json = r#""0200000000010144af9381cd3cb3d14d549b27c8d8a4c87d1d58e501df656342363886277f62e10000000000feffffff02aba9ac0300000000160014908abcc05defb6ba5630268b395b1fab19ad50d760566c0000000000220020c39353c0df01296ab055e83b701715b765636cf91c795deb7573e4b055ada53302473044022010d3b0f0e48977b5c7af7f6a0839a8ed24cd760c4e95668ed7b3275fca727360022007a27825d82a1e69bff2e8cbf195aa4280c214f1cf7650afb6fa2eb49a9765040121036bc4598b0de6ac9c560f1322ce86a0bf27e934837ac86196337db06002c3a352f83a1400""#;

        let tx: SerializedRawTransaction = serde_json::from_str(json).unwrap();

        assert_eq!(tx, SerializedRawTransaction::from(
            "0200000000010144af9381cd3cb3d14d549b27c8d8a4c87d1d58e501df656342363886277f62e10000000000feffffff02aba9ac0300000000160014908abcc05defb6ba5630268b395b1fab19ad50d760566c0000000000220020c39353c0df01296ab055e83b701715b765636cf91c795deb7573e4b055ada53302473044022010d3b0f0e48977b5c7af7f6a0839a8ed24cd760c4e95668ed7b3275fca727360022007a27825d82a1e69bff2e8cbf195aa4280c214f1cf7650afb6fa2eb49a9765040121036bc4598b0de6ac9c560f1322ce86a0bf27e934837ac86196337db06002c3a352f83a1400"));
        let bitcoin_tx: BitcoinTransaction = tx.into();
        let expected_txid = Sha256dHash::from_hex(
            "85a42342de714d4fa39af1fa503b9363df8a31450ff22869b300f686737370e4",
        ).unwrap();
        assert_eq!(bitcoin_tx.txid(), expected_txid);
    }

    #[test]
    fn should_deserialize_verbose_raw_transaction() {
        let json = r#"
        {
          "txid": "96e038ae072e3328cc3fe7dfbac8748127a26335461f8b61bb2082a67c230e38",
          "hash": "b1826b1f6514187abcfcb95cdc870d74125bebaa408e3bab015139990f4c1f5b",
          "version": 2,
          "size": 184,
          "vsize": 157,
          "locktime": 0,
          "vin": [
            {
              "coinbase": "03142d010101",
              "sequence": 4294967295
            }
          ],
          "vout": [
            {
              "value": 0.00000000,
              "n": 0,
              "scriptPubKey": {
                "asm": "039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1 OP_CHECKSIG",
                "hex": "21039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1ac",
                "reqSigs": 1,
                "type": "pubkey",
                "addresses": [
                  "my9XdXbMLZm3v8uqGLuPRKatWjnpXw2boX"
                ]
              }
            },
            {
              "value": 0.00000000,
              "n": 1,
              "scriptPubKey": {
                "asm": "OP_RETURN aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9",
                "hex": "6a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9",
                "type": "nulldata"
              }
            }
          ],
          "hex": "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603142d010101ffffffff0200000000000000002321039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000",
          "blockhash": "796d7a2dbb1213b65dc2f7170575755efdfae8340b2183e971ed5a89113bbedf",
          "confirmations": 9,
          "time": 1525393130,
          "blocktime": 1525393130
        }
        "#;

        let tx: VerboseRawTransaction = serde_json::from_str(json).unwrap();

        assert_eq!(tx, VerboseRawTransaction {
            txid: TransactionId::from_str("96e038ae072e3328cc3fe7dfbac8748127a26335461f8b61bb2082a67c230e38").unwrap(),
            hash: "b1826b1f6514187abcfcb95cdc870d74125bebaa408e3bab015139990f4c1f5b".to_string(),
            size: 184,
            vsize: 157,
            version: 2,
            locktime: 0,
            vin: vec![
                TransactionInput {
                    txid: None,
                    vout: None,
                    script_sig: None,
                    coinbase: Some(String::from("03142d010101")),
                    sequence: 4294967295,
                }
            ],
            vout: vec![
                TransactionOutput {
                    value: 0.0,
                    n: 0,
                    script_pub_key: ScriptPubKey {
                        asm: "039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1 OP_CHECKSIG".to_string(),
                        hex: Script::from(std_hex::decode("21039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1ac").unwrap()),
                        req_sigs: Some(1),
                        script_type: ScriptType::PubKey,
                        addresses: Some(vec![
                            Address::from_str("my9XdXbMLZm3v8uqGLuPRKatWjnpXw2boX").unwrap()
                        ]),
                    },
                },
                TransactionOutput {
                    value: 0.0,
                    n: 1,
                    script_pub_key: ScriptPubKey {
                        asm: "OP_RETURN aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9".to_string(),
                        hex: Script::from(std_hex::decode("6a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9").unwrap()),
                        req_sigs: None,
                        script_type: ScriptType::NullData,
                        addresses: None,
                    },
                }
            ],
            hex: SerializedRawTransaction::from("020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0603142d010101ffffffff0200000000000000002321039b0e80cdda15ac2164392dfaf4f3eb36dd914dcb1c405eec3dd8c9ebf6c13fc1ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000"),
            blockhash: BlockHash::from("796d7a2dbb1213b65dc2f7170575755efdfae8340b2183e971ed5a89113bbedf"),
            confirmations: 9,
            time: 1525393130,
            blocktime: 1525393130,
        })
    }

    #[test]
    fn should_deserialize_unspent_transaction_output() {
        let json = r#"
        {
            "txid" : "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            "vout" : 1,
            "address" : "mgnucj8nYqdrPFh2JfZSB1NmUThUGnmsqe",
            "account" : "test label",
            "scriptPubKey" : "76a9140dfc8bafc8419853b34d5e072ad37d1a5159f58488ac",
            "amount" : 0.00010000,
            "confirmations" : 6210,
            "spendable" : true,
            "solvable" : true
        }
        "#;

        let utxo: UnspentTransactionOutput = serde_json::from_str(json).unwrap();

        assert_eq!(
            utxo,
            UnspentTransactionOutput {
                txid: TransactionId::from_str(
                    "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a"
                ).unwrap(),
                vout: 1,
                address: Some(Address::from_str("mgnucj8nYqdrPFh2JfZSB1NmUThUGnmsqe").unwrap()),
                account: Some(String::from("test label")),
                script_pub_key: Script::from(
                    std_hex::decode("76a9140dfc8bafc8419853b34d5e072ad37d1a5159f58488ac").unwrap()
                ),
                redeem_script: None,
                amount: 0.0001,
                confirmations: 6210,
                spendable: true,
                solvable: true,
                safe: None,
            }
        )
    }

    #[test]
    fn new_transaction_output_should_serialize_to_object() {
        let mut output: NewTransactionOutput = HashMap::new();
        output.insert(
            Address::from_str("mgnucj8nYqdrPFh2JfZSB1NmUThUGnmsqe").unwrap(),
            10.12345,
        );

        let actual_json = serde_json::to_string(&output).unwrap();
        let expected_json = r#"{"mgnucj8nYqdrPFh2JfZSB1NmUThUGnmsqe":10.12345}"#;

        assert_eq!(actual_json, expected_json)
    }
}
