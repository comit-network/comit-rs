use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct Timestamp(SystemTime);

pub fn now() -> Timestamp {
    Timestamp(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_timestamp() {
        let _ts = now();
    }
}
