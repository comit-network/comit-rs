use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Timestamp(u32);

impl Timestamp {
    pub fn now() -> Self {
        Timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime::duration_since failed")
                .as_secs() as u32,
        )
    }

    pub fn plus(self, seconds: u32) -> Self {
        Self(self.0 + seconds)
    }

    pub fn diff(self, other: Timestamp) -> u32 {
        self.0.checked_sub(other.0).unwrap_or(0)
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
