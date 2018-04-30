// TODO: Use a proper struct that represents the actual address format
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Address(String);

from_str!(Address);

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct MultiSigAddress {
    address: Address,
    #[serde(rename = "redeemScript")]
    redeem_script: String,
}

#[cfg(test)]
mod tests {
    use serde_json;
    use super::*;

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
}
