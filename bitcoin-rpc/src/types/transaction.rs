use types::*;

#[derive(Deserialize, Serialize, Debug)]
pub struct TransactionId(String);

impl<'a> From<&'a str> for TransactionId {
    fn from(s: &'a str) -> Self {
        TransactionId(s.to_string())
    }
}

/// Currently the internal representation is the serialized string
/// We might want to have a more sophisticated struct that can de- and encode the tx later on.
/// We will need serializers and deserializers then.
#[derive(Deserialize, Serialize, Debug)]
pub struct RawTransactionHex(String);

impl<'a> From<&'a str> for RawTransactionHex {
    fn from(s: &'a str) -> Self {
        RawTransactionHex(s.to_string())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Transaction {
    amount: f64,
    fee: Option<f64>,
    confirmations: u32,
    blockhash: BlockHash,
    /// Unix Timestamp
    blocktime: u64,
    /// Unix Timestamp
    blockindex: u64,
    txid: TransactionId,
    /// Unix Timestamp
    time: u64,
    /// Unix Timestamp
    timereceived: u64,
    #[serde(rename = "bip125-replaceable")]
    bip125_replaceable: String, // yes|no|unknown: TODO: Create enum if needed
    details: Vec<Detail>,
    hex: RawTransactionHex,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Detail {
    account: String,
    address: Address,
    category: String, // send|receive|immature TODO: Create enum if needed
    amount: f64,
    label: Option<String>,
    vout: u32,
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::*;

    #[test]
    fn should_deserialize_transaction() {
        let tx = r#"{"amount":0.00000000,"confirmations":2,"generated":true,"blockhash":"33ba1550e92f5c73fa852c93d3f32a2ba0931cf64bc38b9be063a5b0f8d07440","blockindex":0,"blocktime":1525055404,"txid":"7e7c52b1f46e7ea2511e885d8c0e5df9297f65b6fff6907ceb1377d0582e45f4","walletconflicts":[],"time":1525055404,"timereceived":1525055404,"bip125-replaceable":"no","details":[{"account":"","address":"n3e8z6HmMDPQGDr3seFjpg88PeagBg2EeR","category":"immature","amount":50.00000000,"vout":0}],"hex":"020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401180101ffffffff0200f2052a01000000232102ec5601272cb71c84d0216661534cfea0d617decbc84a626b7f9f30fb4b0e65d9ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000"}"#;

        let tx: Transaction = serde_json::from_str(tx).unwrap();
    }

}
