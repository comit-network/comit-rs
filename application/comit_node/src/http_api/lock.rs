use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HttpLock {
    unit: String,
    value: u64,
}

impl HttpLock {
    pub fn with_lock(name: &'static str, value: u64) -> HttpLock {
        HttpLock {
            unit: String::from(name),
            value,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Error,
}

macro_rules! impl_to_http_lock {
    ($lock_type:ty, $unit:tt) => {
        impl ToHttpLock for $lock_type {
            fn to_http_lock(&self) -> Result<HttpLock, Error> {
                Ok(HttpLock::with_lock($unit, u64::from(self.0)))
            }
        }
    };
}

pub trait ToHttpLock {
    fn to_http_lock(&self) -> Result<HttpLock, Error>;
}
