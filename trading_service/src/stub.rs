// Types and things that are TODO

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthAddress(pub String);
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthTimeDelta(pub u32); // Measured in seconds
