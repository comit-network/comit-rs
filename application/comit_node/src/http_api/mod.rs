pub mod rfc003;
pub mod route_factory;

#[macro_use]
pub mod ledger;

pub const PATH: &str = "swaps";

use self::ledger::{Error, FromHttpLedger, HttpLedger, ToHttpLedger};
use swap_protocols::ledger::{Bitcoin, Ethereum};

impl_http_ledger!(Bitcoin { network });
impl_http_ledger!(Ethereum);
