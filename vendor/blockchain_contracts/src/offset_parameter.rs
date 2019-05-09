use crate::ethereum::EncodeToEvm;
use std::ops::Range;

#[derive(Debug)]
pub struct OffsetParameter {
    pub value: Vec<u8>,
    pub range: Range<usize>,
}

#[derive(Debug)]
pub enum Error {
    Length(usize, usize),
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

pub fn apply_offsets(template: &[u8], offsets: Vec<OffsetParameter>) -> Vec<u8> {
    let mut contract = template.to_vec();

    for offset in offsets {
        contract.splice(offset.range, offset.value);
    }

    contract
}
