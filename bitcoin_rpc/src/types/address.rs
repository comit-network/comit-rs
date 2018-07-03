use bitcoin;
use bitcoin::util::address::Address as bitcoin_address;
use std::fmt;
use std::str::FromStr;
use types::ScriptType;

// TODO: to use bitcoin::util::address::Address, need to upgrade serde in rust-bitcoin
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Address(String);

impl From<bitcoin_address> for Address {
    fn from(addr: bitcoin_address) -> Self {
        Address::from(addr.to_string().as_str())
    }
}

impl Address {
    pub fn to_bitcoin_address(&self) -> Result<bitcoin_address, bitcoin::util::Error> {
        bitcoin_address::from_str(self.0.as_str())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

#[derive(Debug)]
pub enum Error {
    AddressIsNotBech32,
    BitcoinError(bitcoin::util::Error),
}

impl From<bitcoin::util::Error> for Error {
    fn from(error: bitcoin::util::Error) -> Self {
        Error::BitcoinError(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::AddressIsNotBech32 => write!(f, "address must be bech32"),
            &Error::BitcoinError(_) => write!(f, "address is not in bitcoin format"),
        }
    }
}

from_str!(Address);

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct MultiSigAddress {
    address: Address,
    #[serde(rename = "redeemScript")]
    redeem_script: String,
}

/// Most of the Option<T> are due to different address formats
/// Different fields are returned for P2PKH and P2SH addresses.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct AddressValidationResult {
    #[serde(rename = "isvalid")]
    is_valid: bool,
    address: Option<Address>,
    #[serde(rename = "scriptPubKey")]
    script_pub_key: Option<String>,
    #[serde(rename = "ismine")]
    is_mine: Option<bool>,
    #[serde(rename = "iswatchonly")]
    is_watch_only: Option<bool>,
    #[serde(rename = "isscript")]
    is_script: Option<bool>,
    #[serde(rename = "script")]
    script_type: Option<ScriptType>,
    #[serde(rename = "hex")]
    redeem_script: Option<String>,
    addresses: Option<Vec<Address>>,
    #[serde(rename = "sigsrequired")]
    sigs_required: Option<i32>,
    pubkey: Option<String>, //TODO: use PubkeyHash here
    #[serde(rename = "iscompressed")]
    is_compressed: Option<bool>,
    account: Option<String>,
    #[serde(rename = "hdkeypath")]
    hd_key_path: Option<String>,
    #[serde(rename = "hdmasterkeyid")]
    hd_masterkey_id: Option<String>,
}

#[cfg(test)]
mod tests {
    extern crate hex;

    use super::*;
    use serde_json;

    #[test]
    fn can_deserialize_address() {
        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        struct TestStruct {
            address: Address,
        }

        let address = r#"{"address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"}"#;

        let test_struct: TestStruct = serde_json::from_str(address).unwrap();

        assert_eq!(
            test_struct,
            TestStruct {
                address: Address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string()),
            }
        )
    }

    #[test]
    fn can_deserialize_p2pkh_validation_result() {
        let json = r#"
        {
            "isvalid": true,
            "address": "17fshh33qUze2yifiJ2sXgijSMzJ2KNEwu",
            "scriptPubKey": "76a914492ae280d70af33acf0ae7cd329b961e65e9cbd888ac",
            "ismine": true,
            "iswatchonly": false,
            "isscript": false,
            "pubkey": "0312eeb9ae5f14c3cf43cece11134af860c2ef7d775060e3a578ceec888acada31",
            "iscompressed": true,
            "account": "Test"
        }
"#;

        let result: AddressValidationResult = serde_json::from_str(json).unwrap();

        assert_eq!(
            result,
            AddressValidationResult {
                is_valid: true,
                address: Some(Address::from("17fshh33qUze2yifiJ2sXgijSMzJ2KNEwu")),
                script_pub_key: Some(String::from(
                    "76a914492ae280d70af33acf0ae7cd329b961e65e9cbd888ac"
                )),
                is_mine: Some(true),
                is_watch_only: Some(false),
                is_script: Some(false),
                script_type: None,
                redeem_script: None,
                addresses: None,
                sigs_required: None,
                pubkey: Some(String::from(
                    "0312eeb9ae5f14c3cf43cece11134af860c2ef7d775060e3a578ceec888acada31"
                )),
                is_compressed: Some(true),
                account: Some(String::from("Test")),
                hd_key_path: None,
                hd_masterkey_id: None,
            }
        )
    }

    #[test]
    fn can_deserialize_p2sh_validation_result() {
        let json = r#"
        {
            "isvalid" : true,
            "address" : "2MyVxxgNBk5zHRPRY2iVjGRJHYZEp1pMCSq",
            "ismine" : true,
            "iswatchonly" : false,
            "isscript" : true,
            "script" : "multisig",
            "hex" : "522103ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf2102ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f21022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d553ae",
            "addresses" : [
                "mjbLRSidW1MY8oubvs4SMEnHNFXxCcoehQ",
                "mo1vzGwCzWqteip29vGWWW6MsEBREuzW94",
                "mt17cV37fBqZsnMmrHnGCm9pM28R1kQdMG"
            ],
            "sigsrequired" : 2,
            "account" : "test account"
        }
"#;

        let result: AddressValidationResult = serde_json::from_str(json).unwrap();

        assert_eq!(result, AddressValidationResult {
            is_valid: true,
            address: Some(Address::from("2MyVxxgNBk5zHRPRY2iVjGRJHYZEp1pMCSq")),
            script_pub_key: None,
            is_mine: Some(true),
            is_watch_only: Some(false),
            is_script: Some(true),
            script_type: Some(ScriptType::MultiSig),
            redeem_script: Some(String::from("522103ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf2102ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f21022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d553ae")),
            addresses: Some(vec![
                Address::from("mjbLRSidW1MY8oubvs4SMEnHNFXxCcoehQ"),
                Address::from("mo1vzGwCzWqteip29vGWWW6MsEBREuzW94"),
                Address::from("mt17cV37fBqZsnMmrHnGCm9pM28R1kQdMG"),
            ]),
            sigs_required: Some(2),
            pubkey: None,
            is_compressed: None,
            account: Some(String::from("test account")),
            hd_key_path: None,
            hd_masterkey_id: None,
        })
    }
}
