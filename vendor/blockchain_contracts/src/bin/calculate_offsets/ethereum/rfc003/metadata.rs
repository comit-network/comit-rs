pub struct Metadata {
    pub ledger_name: String,
    pub asset_name: String,
    pub contract_hex: String,
}

impl Metadata {
    pub fn to_markdown(&self) -> String {
        format!(
            "** {} on {} **\nContract template:\n {}",
            self.asset_name, self.ledger_name, self.contract_hex
        )
    }
}
