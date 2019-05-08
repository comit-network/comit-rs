use crate::ethereum::{EncodeToEvm, ToEvmError};
use std::ops::Range;

#[derive(Debug)]
pub struct OffsetParameter {
    pub value: Vec<u8>,
    pub range: Range<usize>,
}

#[derive(Debug)]
pub enum Error {
    ToEVM(ToEvmError),
    Length(usize, usize),
}

impl OffsetParameter {
    pub fn new<T>(value: T, range: Range<usize>) -> Result<OffsetParameter, Error>
    where
        T: EncodeToEvm,
    {
        let value = value.encode_to_evm().map_err(Error::ToEVM)?;

        if value.len() != range.len() {
            return Err(Error::Length(value.len(), range.len()));
        }

        Ok(OffsetParameter { value, range })
    }
}
