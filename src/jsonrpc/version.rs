#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub enum Version {
    #[serde(rename = "1.0")]
    V1,

    #[serde(rename = "2.0")]
    V2,
}
