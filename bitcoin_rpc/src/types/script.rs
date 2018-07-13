use bitcoin::blockdata::script::Script;
use types::*;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct RedeemScript(String);

from_str!(RedeemScript);

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct ScriptPubKey {
    pub asm: String,
    #[serde(with = "self::serde::script")]
    pub hex: Script,
    #[serde(rename = "reqSigs")]
    pub req_sigs: Option<u32>,
    #[serde(rename = "type")]
    pub script_type: ScriptType,
    pub addresses: Option<Vec<Address>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum ScriptType {
    #[serde(rename = "pubkey")]
    PubKey,
    #[serde(rename = "pubkeyhash")]
    PubKeyHash,
    #[serde(rename = "multisig")]
    MultiSig,
    #[serde(rename = "nonstandard")]
    NonStandard,
    #[serde(rename = "witness_v0_keyhash")]
    WitnessPubKeyHash,
    /// Appears for generated transactions
    #[serde(rename = "nulldata")]
    NullData,
    #[serde(rename = "witness_v0_scripthash")]
    WitnessScriptHash,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct DecodedScript {
    asm: String,
    #[serde(rename = "type")]
    script_type: Option<ScriptType>,
    #[serde(rename = "reqSigs")]
    req_sigs: Option<u32>,
    addresses: Option<Vec<Address>>,
    p2sh: Address,
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn can_deserialize_decoded_script_type() {
        let json = r#"
        {
            "asm" : "2 03ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf 02ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f 022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d5 3 OP_CHECKMULTISIG",
            "reqSigs" : 2,
            "type" : "multisig",
            "addresses" : [
                "mjbLRSidW1MY8oubvs4SMEnHNFXxCcoehQ",
                "mo1vzGwCzWqteip29vGWWW6MsEBREuzW94",
                "mt17cV37fBqZsnMmrHnGCm9pM28R1kQdMG"
            ],
            "p2sh" : "2MyVxxgNBk5zHRPRY2iVjGRJHYZEp1pMCSq"
        }"#;

        let script: DecodedScript = serde_json::from_str(json).unwrap();

        assert_eq!(script, DecodedScript {
            asm: "2 03ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf 02ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f 022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d5 3 OP_CHECKMULTISIG".to_string(),
            script_type: Some(ScriptType::MultiSig),
            req_sigs: Some(2),
            addresses: Some(vec![
                Address::from_str("mjbLRSidW1MY8oubvs4SMEnHNFXxCcoehQ").unwrap(),
                Address::from_str("mo1vzGwCzWqteip29vGWWW6MsEBREuzW94").unwrap(),
                Address::from_str("mt17cV37fBqZsnMmrHnGCm9pM28R1kQdMG").unwrap(),
            ]),
            p2sh: Address::from_str("2MyVxxgNBk5zHRPRY2iVjGRJHYZEp1pMCSq").unwrap(),
        })
    }

}
