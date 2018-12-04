use std::path::Path;

pub trait FromFile
where
    Self: std::marker::Sized,
{
    type Err;

    fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, Self::Err>;
}
