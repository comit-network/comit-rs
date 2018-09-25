mod client;
mod factory;
mod fake_client;
pub use self::{
    client::{Client, DefaultClient, SwapReject},
    factory::{DefaultFactory, Factory, FactoryError, FakeFactory},
    fake_client::FakeClient,
};
