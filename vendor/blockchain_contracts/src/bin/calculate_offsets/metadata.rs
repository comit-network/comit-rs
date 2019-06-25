pub struct Metadata {
    pub ledger_name: String,
    pub asset_name: String,
    pub contract: Vec<u8>,
}

impl Metadata {
    pub fn to_markdown(&self) -> String {
        format!(
            "** {} on {} **\nContract template:\n {}",
            self.asset_name,
            self.ledger_name,
            hex::encode(&self.contract)
        )
    }
}
