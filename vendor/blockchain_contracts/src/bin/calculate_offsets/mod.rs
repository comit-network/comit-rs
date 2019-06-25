use crate::calculate_offsets::{metadata::Metadata, offset::Offset};
use std::ffi::OsStr;

pub mod ethereum;
pub mod metadata;
pub mod offset;

pub trait Contract: std::marker::Sized {
    type Error;

    fn compile<S: AsRef<OsStr>>(template_folder: S) -> Result<Self, Self::Error>;
    fn placeholder_offsets(&self) -> Result<Vec<Offset>, Self::Error>;
    fn meta_data(&self) -> Metadata;
}
