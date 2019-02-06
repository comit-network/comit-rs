use crate::{
    http_api::{
        self,
        asset::{HttpAsset, ToHttpAsset},
        ledger::{HttpLedger, ToHttpLedger},
        problem::{self, HttpApiProblemStdError},
        rfc003::socket_addr,
    },
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            alice::{AliceSpawner, SwapRequestIdentities},
            state_store::StateStore,
            Actions, Alice, Bob, Ledger, SecretSource, Timestamp,
        },
        Metadata, MetadataStore, RoleKind, SwapId,
    },
};
use bitcoin_support::{self, BitcoinQuantity};
use ethereum_support::{self, Erc20Token, EtherQuantity};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::{net::SocketAddr, sync::Arc};
use warp::{self, Rejection, Reply};



//#[cfg(test)]
//mod tests {
//
//    use super::*;
//    use spectral::prelude::*;
//    use std::net::{IpAddr, Ipv4Addr};
//
//    #[test]
//    fn can_deserialize_swap_request_body_with_port() {
//        let body = r#"{
//                "alpha_ledger": {
//                    "name": "Bitcoin",
//                    "network": "regtest"
//                },
//                "beta_ledger": {
//                    "name": "Ethereum",
//                    "network": "regtest"
//                },
//                "alpha_asset": {
//                    "name": "Bitcoin",
//                    "quantity": "100000000"
//                },
//                "beta_asset": {
//                    "name": "Ether",
//                    "quantity": "10000000000000000000"
//                },
//                "alpha_ledger_refund_identity": null,
//                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
//                "alpha_expiry": 2000000000,
//                "beta_expiry": 2000000000,
//                "peer": "127.0.0.1:8002"
//            }"#;
//
//        let body = serde_json::from_str(body);
//
//        assert_that(&body).is_ok_containing(SwapRequestBody {
//            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
//            beta_asset: EtherQuantity::from_eth(10.0),
//            alpha_ledger: Bitcoin::default(),
//            beta_ledger: Ethereum::default(),
//            alpha_expiry: Timestamp::from(2000000000),
//            beta_expiry: Timestamp::from(2000000000),
//            identities: SwapRequestBodyIdentities::OnlyRedeem {
//                beta_ledger_redeem_identity: ethereum_support::Address::from(
//                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
//                ),
//            },
//            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
//        })
//    }
//
//    #[test]
//    fn can_deserialize_swap_request_body_without_port() {
//        let body = r#"{
//                "alpha_ledger": {
//                    "name": "Bitcoin",
//                    "network": "regtest"
//                },
//                "beta_ledger": {
//                    "name": "Ethereum",
//                    "network": "regtest"
//                },
//                "alpha_asset": {
//                    "name": "Bitcoin",
//                    "quantity": "100000000"
//                },
//                "beta_asset": {
//                    "name": "Ether",
//                    "quantity": "10000000000000000000"
//                },
//                "alpha_ledger_refund_identity": null,
//                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
//                "alpha_expiry": 2000000000,
//                "beta_expiry": 2000000000,
//                "peer": "127.0.0.1"
//            }"#;
//
//        let body = serde_json::from_str(body);
//
//        assert_that(&body).is_ok_containing(SwapRequestBody {
//            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
//            beta_asset: EtherQuantity::from_eth(10.0),
//            alpha_ledger: Bitcoin::default(),
//            beta_ledger: Ethereum::default(),
//            alpha_expiry: Timestamp::from(2000000000),
//            beta_expiry: Timestamp::from(2000000000),
//            identities: SwapRequestBodyIdentities::OnlyRedeem {
//                beta_ledger_redeem_identity: ethereum_support::Address::from(
//                    "0x00a329c0648769a73afac7f9381e08fb43dbea72",
//                ),
//            },
//            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9939),
//        })
//    }
//
//}
