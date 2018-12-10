use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HttpLockDuration {
    #[serde(rename = "type")]
    lock_type: String,
    value: u64,
}

impl HttpLockDuration {
    pub fn with_lock_duration(name: &'static str, value: u64) -> HttpLockDuration {
        HttpLockDuration {
            lock_type: String::from(name).to_lowercase(),
            value,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Error,
}

macro_rules! impl_to_http_lock_duration {
    ($lock_type:ty) => {
        impl ToHttpLockDuration for $lock_type {
            fn to_http_lock_duration(&self) -> Result<HttpLockDuration, Error> {
                Ok(HttpLockDuration::with_lock_duration(
                    stringify!($lock_type),
                    u64::from(self.0),
                ))
            }
        }
    };
}

pub trait ToHttpLockDuration {
    fn to_http_lock_duration(&self) -> Result<HttpLockDuration, Error>;
}
