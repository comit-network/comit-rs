#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Regtest,
    Ropsten,
}
