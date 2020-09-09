mod action;
mod dial_addr;
pub mod halbit;
mod halbit_herc20;
pub mod hbit;
mod hbit_herc20;
pub mod herc20;
mod herc20_halbit;
mod herc20_hbit;
mod info;
mod markets;
mod orders;
mod peers;
mod problem;
mod route_factory;
mod serde_peer_id;
mod swaps;
mod tokens;

pub use self::{
    halbit::Halbit, hbit::Hbit, herc20::Herc20, problem::*, route_factory::create as create_routes,
};

pub const PATH: &str = "swaps";

use crate::{
    asset,
    asset::Erc20Quantity,
    ethereum,
    storage::{tables, BtcDaiOrder, CreatedSwap, Order},
    LocalSwapId, Role, Secret, SecretHash, Timestamp,
};
use anyhow::Result;
use chrono::Utc;
use comit::{OrderId, Position, Price, Quantity};
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use warp::http::Method;

/// Object representing the data of a POST request for creating a swap.
#[derive(Deserialize, Clone, Debug)]
pub struct PostBody<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Role,
}

impl<A, B> PostBody<A, B> {
    pub fn to_created_swap<CA, CB>(&self, swap_id: LocalSwapId) -> CreatedSwap<CA, CB>
    where
        CA: From<A>,
        CB: From<B>,
        Self: Clone,
    {
        let body = self.clone();

        let alpha = CA::from(body.alpha);
        let beta = CB::from(body.beta);

        let start_of_swap = Utc::now().naive_local();

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role,
            start_of_swap,
        }
    }
}

/// The struct representing the properties within the siren document in our
/// response.
#[derive(Serialize)]
struct OrderProperties {
    id: OrderId,
    position: Position,
    price: Amount,
    quantity: Amount,
    state: State,
}

impl From<(tables::Order, tables::BtcDaiOrder)> for OrderProperties {
    fn from(tuple: (Order, BtcDaiOrder)) -> Self {
        let (order, btc_dai_order) = tuple;

        Self {
            id: order.order_id,
            position: order.position,
            price: Amount::from(btc_dai_order.price),
            quantity: Amount::from(btc_dai_order.quantity),
            state: State {
                open: btc_dai_order.open.to_inner(),
                closed: btc_dai_order.closed.to_inner(),
                settling: btc_dai_order.settling.to_inner(),
                failed: btc_dai_order.failed.to_inner(),
                cancelled: btc_dai_order.cancelled.to_inner(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "currency")]
pub enum Amount {
    #[serde(rename = "BTC")]
    Bitcoin {
        #[serde(with = "asset::bitcoin::sats_as_string")]
        value: asset::Bitcoin,
        decimals: u8,
    },
    #[serde(rename = "DAI")]
    Dai { value: Erc20Quantity, decimals: u8 },
}

impl From<Quantity<asset::Bitcoin>> for Amount {
    fn from(quantity: Quantity<asset::Bitcoin>) -> Self {
        Amount::btc(quantity.to_inner())
    }
}

impl From<Price<asset::Bitcoin, Erc20Quantity>> for Amount {
    fn from(price: Price<asset::Bitcoin, Erc20Quantity>) -> Self {
        Amount::dai(price.wei_per_btc())
    }
}

impl Amount {
    fn btc(value: asset::Bitcoin) -> Self {
        Amount::Bitcoin { value, decimals: 8 }
    }

    fn dai(value: Erc20Quantity) -> Self {
        Amount::Dai {
            value,
            decimals: 18,
        }
    }
}

#[derive(Serialize)]
struct State {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    open: asset::Bitcoin,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    closed: asset::Bitcoin,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    settling: asset::Bitcoin,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    failed: asset::Bitcoin,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    cancelled: asset::Bitcoin,
}

impl State {
    pub fn is_open(&self) -> bool {
        self.open != asset::Bitcoin::ZERO
    }
}

fn make_order_entity(properties: OrderProperties) -> Result<siren::Entity> {
    let mut entity = siren::Entity::default().with_properties(&properties)?;

    if let Some(action) = cancel_action(&properties) {
        entity = entity.with_action(action)
    }

    Ok(entity)
}

fn cancel_action(order: &OrderProperties) -> Option<siren::Action> {
    if order.state.is_open() {
        Some(siren::Action {
            name: "cancel".to_string(),
            class: vec![],
            method: Some(Method::DELETE),
            href: format!("/orders/{}", order.id),
            title: None,
            _type: Some("application/json".to_owned()),
            fields: vec![],
        })
    } else {
        None
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "protocol")]
pub enum Protocol {
    Hbit { asset: Amount },
    Herc20 { asset: Amount },
    Halbit { asset: Amount },
}

impl Protocol {
    pub fn hbit(btc: asset::Bitcoin) -> Self {
        Protocol::Hbit {
            asset: Amount::btc(btc),
        }
    }

    pub fn halbit(btc: asset::Bitcoin) -> Self {
        Protocol::Halbit {
            asset: Amount::btc(btc),
        }
    }

    pub fn herc20_dai(dai: Erc20Quantity) -> Self {
        Protocol::Herc20 {
            asset: Amount::dai(dai),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActionName {
    Init,
    Deploy,
    Fund,
    Redeem,
    Refund,
}

pub trait Events {
    fn events(&self) -> Vec<SwapEvent>;
}

/// Get the underlying ledger used by the alpha protocol.
pub trait AlphaLedger {
    fn alpha_ledger(&self) -> Ledger;
}

/// Get the underlying ledger used by the beta protocol.
pub trait BetaLedger {
    fn beta_ledger(&self) -> Ledger;
}

/// Get the absolute expiry time for the alpha protocol.
pub trait AlphaAbsoluteExpiry {
    fn alpha_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Get the absolute expiry time for the beta protocol.
pub trait BetaAbsoluteExpiry {
    fn beta_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Ledgers we currently support swaps on top of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ledger {
    Bitcoin,
    Ethereum,
}

pub trait GetRole {
    fn get_role(&self) -> Role;
}

pub trait AlphaProtocol {
    fn alpha_protocol(&self) -> Protocol;
}

pub trait BetaProtocol {
    fn beta_protocol(&self) -> Protocol;
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum SwapEvent {
    HbitFunded { tx: bitcoin::Txid },
    HbitIncorrectlyFunded { tx: bitcoin::Txid },
    HbitRedeemed { tx: bitcoin::Txid },
    HbitRefunded { tx: bitcoin::Txid },
    Herc20Deployed { tx: ethereum::Hash },
    Herc20Funded { tx: ethereum::Hash },
    Herc20IncorrectlyFunded { tx: ethereum::Hash },
    Herc20Redeemed { tx: ethereum::Hash },
    Herc20Refunded { tx: ethereum::Hash },

    // TODO: Seriously reconsider this naming + the whole halbit protocol design in general. The
    // event-based design here should allow us to name this whatever and hence make it more
    // descriptive.
    HalbitFunded,
    HalbitIncorrectlyFunded,
    HalbitRedeemed,
    HalbitRefunded,
}

impl From<&herc20::State> for Vec<SwapEvent> {
    fn from(state: &herc20::State) -> Self {
        match state {
            herc20::State::None => vec![],
            herc20::State::Deployed {
                deploy_transaction, ..
            } => vec![SwapEvent::Herc20Deployed {
                tx: deploy_transaction.hash,
            }],
            herc20::State::Funded {
                deploy_transaction,
                fund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
            ],
            herc20::State::IncorrectlyFunded {
                deploy_transaction,
                fund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20IncorrectlyFunded {
                    tx: fund_transaction.hash,
                },
            ],
            herc20::State::Redeemed {
                deploy_transaction,
                fund_transaction,
                redeem_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
                SwapEvent::Herc20Redeemed {
                    tx: redeem_transaction.hash,
                },
            ],
            herc20::State::Refunded {
                deploy_transaction,
                fund_transaction,
                refund_transaction,
                ..
            } => vec![
                SwapEvent::Herc20Deployed {
                    tx: deploy_transaction.hash,
                },
                SwapEvent::Herc20Funded {
                    tx: fund_transaction.hash,
                },
                SwapEvent::Herc20Refunded {
                    tx: refund_transaction.hash,
                },
            ],
        }
    }
}

impl From<&hbit::State> for Vec<SwapEvent> {
    fn from(state: &hbit::State) -> Self {
        match state {
            hbit::State::None => vec![],
            hbit::State::Funded {
                fund_transaction, ..
            } => vec![SwapEvent::HbitFunded {
                tx: fund_transaction.txid(),
            }],
            hbit::State::IncorrectlyFunded {
                fund_transaction, ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitIncorrectlyFunded {
                    tx: fund_transaction.txid(),
                },
            ],
            hbit::State::Redeemed {
                fund_transaction,
                redeem_transaction,
                ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitRedeemed {
                    tx: redeem_transaction.txid(),
                },
            ],
            hbit::State::Refunded {
                fund_transaction,
                refund_transaction,
                ..
            } => vec![
                SwapEvent::HbitFunded {
                    tx: fund_transaction.txid(),
                },
                SwapEvent::HbitRefunded {
                    tx: refund_transaction.txid(),
                },
            ],
        }
    }
}

impl From<&halbit::State> for Vec<SwapEvent> {
    fn from(state: &halbit::State) -> Self {
        match state {
            halbit::State::None => vec![],
            halbit::State::Opened(_) => vec![],
            halbit::State::Accepted(_) => vec![SwapEvent::HalbitFunded],
            halbit::State::Settled(_) => vec![SwapEvent::HalbitFunded, SwapEvent::HalbitRedeemed],
            halbit::State::Cancelled(_) => vec![SwapEvent::HalbitFunded, SwapEvent::HalbitRefunded],
        }
    }
}

#[derive(Clone, Debug)]
pub enum AliceSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret: Secret,
    },
}

#[derive(Clone, Debug)]
pub enum BobSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret_hash: SecretHash,
    },
}

impl<AC, BC, AF, BF> GetRole for AliceSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl<AC, BC, AF, BF> GetRole for BobSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DialInformation {
    JustPeerId(#[serde(with = "serde_peer_id")] PeerId),
    WithAddressHint {
        #[serde(with = "serde_peer_id")]
        peer_id: PeerId,
        address_hint: Multiaddr,
    },
}

impl DialInformation {
    fn into_peer_with_address_hint(self) -> (PeerId, Option<Multiaddr>) {
        match self {
            DialInformation::JustPeerId(inner) => (inner, None),
            DialInformation::WithAddressHint {
                peer_id,
                address_hint,
            } => (peer_id, Some(address_hint)),
        }
    }
}

impl From<DialInformation> for PeerId {
    fn from(dial_information: DialInformation) -> Self {
        match dial_information {
            DialInformation::JustPeerId(inner) => inner,
            DialInformation::WithAddressHint { peer_id, .. } => peer_id,
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("action not found")]
pub struct ActionNotFound;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        asset::{ethereum::FromWei, Bitcoin},
    };
    use uuid::Uuid;

    #[test]
    fn response_serializes_correctly() {
        let properties = OrderProperties {
            id: OrderId::from(Uuid::from_u128(0)),
            position: Position::Sell,
            price: Amount::dai(Erc20Quantity::from_wei_dec_str("9100000000000000000000").unwrap()),
            quantity: Amount::btc(Bitcoin::from_sat(10000000)),
            state: State {
                open: Bitcoin::from_sat(3000000),
                closed: Bitcoin::from_sat(1000000),
                settling: Bitcoin::from_sat(0),
                failed: Bitcoin::from_sat(6000000),
                cancelled: Bitcoin::from_sat(0),
            },
        };

        let result = serde_json::to_string_pretty(&properties).unwrap();

        assert_eq!(
            result,
            r#"{
  "id": "00000000-0000-0000-0000-000000000000",
  "position": "sell",
  "price": {
    "currency": "DAI",
    "value": "9100000000000000000000",
    "decimals": 18
  },
  "quantity": {
    "currency": "BTC",
    "value": "10000000",
    "decimals": 8
  },
  "state": {
    "open": "3000000",
    "closed": "1000000",
    "settling": "0",
    "failed": "6000000",
    "cancelled": "0"
  }
}"#
        );
    }

    #[test]
    fn btc_amount_serializes_properly() {
        let amount = Amount::btc(asset::Bitcoin::from_sat(100000000));

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"BTC","value":"100000000","decimals":8}"#
        )
    }

    #[test]
    fn dai_amount_serializes_properly() {
        let amount =
            Amount::dai(Erc20Quantity::from_wei_dec_str("9000000000000000000000").unwrap());

        let string = serde_json::to_string(&amount).unwrap();

        assert_eq!(
            string,
            r#"{"currency":"DAI","value":"9000000000000000000000","decimals":18}"#
        )
    }

    #[test]
    fn hbit_protocol_serializes_correctly() {
        let protocol = Protocol::hbit(asset::Bitcoin::from_sat(10_000));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "hbit",
  "asset": {
    "currency": "BTC",
    "value": "10000",
    "decimals": 8
  }
}"#
        )
    }

    #[test]
    fn halbit_protocol_serializes_correctly() {
        let protocol = Protocol::halbit(asset::Bitcoin::from_sat(10_000));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "halbit",
  "asset": {
    "currency": "BTC",
    "value": "10000",
    "decimals": 8
  }
}"#
        )
    }

    #[test]
    fn herc20_protocol_serializes_correctly() {
        let protocol = Protocol::herc20_dai(Erc20Quantity::from_wei(1_000_000_000_000_000u64));

        let result = serde_json::to_string_pretty(&protocol).unwrap();

        assert_eq!(
            result,
            r#"{
  "protocol": "herc20",
  "asset": {
    "currency": "DAI",
    "value": "1000000000000000",
    "decimals": 18
  }
}"#
        )
    }
}
