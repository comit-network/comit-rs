use std::time::{Duration, SystemTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Timestamp(u32);

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

impl From<Timestamp> for Duration {
    fn from(item: Timestamp) -> Self {
        Duration::from_secs(u64::from(item.0))
    }
}

impl From<Timestamp> for i64 {
    fn from(item: Timestamp) -> Self {
        i64::from(item.0)
    }
}

impl Timestamp {
    pub fn after(seconds: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed")
            .as_secs() as u32;

        (now + seconds).into()
    }

    pub fn now() -> Self {
        Timestamp::after(0)
    }
}
