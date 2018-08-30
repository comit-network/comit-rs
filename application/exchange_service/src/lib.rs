#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![cfg_attr(test, deny(warnings))]

extern crate bitcoin_rpc_client;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate common_types;
extern crate ethereum_htlc;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate secp256k1_support;
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate event_store;
extern crate logging;

pub mod bitcoin_fee_service;
pub mod bitcoin_service;
pub mod ethereum_service;
pub mod gas_price_service;
pub mod ledger_htlc_service;
pub mod rocket_factory;
pub mod swaps;
pub mod treasury_api_client;
