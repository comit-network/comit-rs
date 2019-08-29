use bitcoin_support::OutPoint;
use serde::ser::{SerializeStruct, Serializer};

pub fn serialize<S: Serializer>(
    value: &Option<OutPoint>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => {
            let mut state = serializer.serialize_struct("OutPoint", 2)?;
            state.serialize_field("txid", &value.txid)?;
            state.serialize_field("vout", &value.vout)?;
            state.end()
        }
        None => serializer.serialize_none(),
    }
}
