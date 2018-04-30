use types::*;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TransactionId(String);

from_str!(TransactionId);

/// Currently the internal representation is the serialized string
/// We might want to have a more sophisticated struct that can de- and encode the tx later on.
/// We will need serializers and deserializers then.
#[derive(Deserialize, Serialize, Debug)]
pub struct RawTransactionHex(String);

from_str!(RawTransactionHex);

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
    bip125_replaceable: String,
    // yes|no|unknown: TODO: Create enum if needed
    details: Vec<Detail>,
    hex: RawTransactionHex,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Detail {
    account: String,
    address: Option<Address>,
    category: String,
    // send|receive|immature|generate|orphan TODO: Create enum if needed
    amount: f64,
    fee: Option<f64>,
    vout: u32,
    #[serde(rename = "involvesWatchonly")]
    involves_watchonly: Option<bool>,
    abandoned: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct DecodedRawTransaction {
    txid: TransactionId,
    hash: String,
    size: u32,
    vsize: u32,
    version: u32,
    locktime: u32,
    vin: Vec<TransactionInput>,
    vout: Vec<TransactionOutput>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ScriptSig {
    asm: String,
    hex: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TransactionInput {
    txid: TransactionId,
    vout: u32,
    #[serde(rename = "scriptSig")]
    script_sig: ScriptSig,
    sequence: u64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ScriptPubKey {
    asm: String,
    hex: String,
    #[serde(rename = "reqSigs")]
    req_sigs: u32,
    #[serde(rename = "type")]
    script_type: ScriptType,
    addresses: Vec<Address>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TransactionOutput {
    value: f64,
    n: u32,
    #[serde(rename = "scriptPubKey")]
    script_pub_key: ScriptPubKey,
}

#[cfg(test)]
mod tests {
    use serde_json;
    use super::*;

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
            txid: TransactionId::from("52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46"),
            hash: "52309405287e737cf412fc42883d65a392ab950869fae80b2a5f1e33326aca46".to_string(),
            size: 223,
            vsize: 223,
            version: 1,
            locktime: 0,
            vin: vec![
                TransactionInput {
                    txid: TransactionId::from("2ac0daff49a4ff82a35a4864797f99f23c396b0529c5ba1e04b3d7b97521feba"),
                    vout: 0,
                    script_sig: ScriptSig {
                        asm: "3044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b[ALL] 0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8".to_string(),
                        hex: "473044022013d212c22f0b46bb33106d148493b9a9723adb2c3dd3a3ebe3a9c9e3b95d8cb00220461661710202fbab550f973068af45c294667fc4dc526627a7463eb23ab39e9b01410479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8".to_string(),
                    },
                    sequence: 4294967295,
                }
            ],
            vout: vec![
                TransactionOutput {
                    value: 0.06990000,
                    n: 0,
                    script_pub_key: ScriptPubKey {
                        asm: "OP_DUP OP_HASH160 01b81d5fa1e55e069e3cc2db9c19e2e80358f306 OP_EQUALVERIFY OP_CHECKSIG".to_string(),
                        hex: "76a91401b81d5fa1e55e069e3cc2db9c19e2e80358f30688ac".to_string(),
                        req_sigs: 1,
                        script_type: ScriptType::PubKeyHash,
                        addresses: vec![
                            Address::from("1A6Ei5cRfDJ8jjhwxfzLJph8B9ZEthR9Z")
                        ],
                    },
                }
            ],
        })
    }
}
