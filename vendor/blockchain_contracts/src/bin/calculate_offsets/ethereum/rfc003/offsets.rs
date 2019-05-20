use crate::calculate_offsets::ethereum::rfc003::offset::Offset;

pub struct Offsets {
    pub ledger_name: String,
    pub asset_name: String,
    pub contract: String,
    pub offsets: Vec<Offset>,
}

impl Offsets {
    pub fn new(
        ledger_name: String,
        asset_name: String,
        contract: String,
        offsets: Vec<Offset>,
    ) -> Offsets {
        Offsets {
            ledger_name,
            asset_name,
            contract,
            offsets,
        }
    }
}
