pub struct Metadata {
    pub ledger_name: String,
    pub asset_name: String,
    pub contract_hex: String,
}

impl Metadata {
    pub fn new(ledger_name: String, asset_name: String, contract_hex: String) -> Metadata {
        Metadata {
            ledger_name,
            asset_name,
            contract_hex,
        }
    }
}
