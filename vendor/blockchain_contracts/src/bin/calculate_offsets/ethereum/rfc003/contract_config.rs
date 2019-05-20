use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ContractConfig {
    pub ledger_name: String,
    pub asset_name: String,
    pub placeholders: Vec<Placeholder>,
}

#[derive(Debug, Deserialize)]
pub struct Placeholder {
    pub name: String,
    pub replace_pattern: String,
}
