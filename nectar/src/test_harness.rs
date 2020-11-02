#[cfg(feature = "testcontainers")]
pub mod bitcoin;
#[cfg(feature = "testcontainers")]
pub mod ethereum;

/// A trait that provide a static stub value for testing purposes
pub trait StaticStub {
    fn static_stub() -> Self;
}
