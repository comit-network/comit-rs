use byteorder::{BigEndian, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Timestamp(u32);

#[derive(Debug)]
pub enum ToVecError {
    ValueTooLong,
    Io(std::io::Error),
}

impl Timestamp {
    // This will work for the next 20 years
    #[allow(clippy::cast_possible_truncation)]
    pub fn now() -> Self {
        Timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime::duration_since failed")
                .as_secs() as u32,
        )
    }

    pub fn plus(self, seconds: u32) -> Self {
        Self(self.0.checked_add(seconds).unwrap_or(std::u32::MAX))
    }

    pub fn to_vec(self, len: usize) -> Result<Vec<u8>, ToVecError> {
        let mut vec = Vec::with_capacity(len);
        vec.write_u32::<BigEndian>(self.0).map_err(ToVecError::Io)?;

        if vec.len() > len {
            return Err(ToVecError::ValueTooLong);
        } else if vec.len() < len {
            let mut temp = Vec::with_capacity(len);
            temp.copy_from_slice(vec.as_slice());
            let delta = len - vec.len();
            for _ in 1..delta {
                temp.push(0);
            }
            for item in vec.iter().skip(delta) {
                temp.push(*item)
            }

            vec.copy_from_slice(temp.as_slice());
        }

        Ok(vec)
    }
}

impl From<u32> for Timestamp {
    fn from(item: u32) -> Self {
        Self(item)
    }
}

impl From<Timestamp> for u32 {
    fn from(item: Timestamp) -> Self {
        item.0
    }
}

impl From<Timestamp> for i64 {
    fn from(item: Timestamp) -> Self {
        i64::from(item.0)
    }
}
