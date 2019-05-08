use crate::ethereum::EncodeToEvm;
use hex::FromHexError;
use std::ops::Range;

#[derive(Debug)]
pub struct OffsetParameter {
    pub value: Vec<u8>,
    pub range: Range<usize>,
}

#[derive(Debug)]
pub enum Error {
    Length(usize, usize),
    FromHex(FromHexError),
}

impl OffsetParameter {
    pub fn new<T>(value: T, range: Range<usize>) -> Result<OffsetParameter, Error>
    where
        T: EncodeToEvm,
    {
        let value = value.encode_to_evm();

        if value.len() != range.len() {
            return Err(Error::Length(value.len(), range.len()));
        }

        Ok(OffsetParameter { value, range })
    }
}

pub fn apply_offsets(template: &str, offsets: Vec<OffsetParameter>) -> Result<Vec<u8>, Error> {
    let mut data = hex::decode(template).map_err(Error::FromHex)?;

    for offset in offsets {
        data.splice(offset.range, offset.value);
    }

    Ok(data)
}
