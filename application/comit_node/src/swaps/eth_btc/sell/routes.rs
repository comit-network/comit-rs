use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    secret::Secret,
};
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use ledger_htlc_service::{self, LedgerHtlcService};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swaps::{
    errors::Error,
};
