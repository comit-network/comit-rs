#[allow(unused_imports)]
use reqwest;

mod client;
mod fake_client;

pub use self::{client::*, fake_client::*};
