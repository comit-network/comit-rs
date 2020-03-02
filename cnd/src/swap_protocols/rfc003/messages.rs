use crate::{
    asset::{self, AssetKind},
    bitcoin::PublicKey,
    comit_api::LedgerKind,
    identity,
    libp2p_comit_ext::ToHeader,
    swap_protocols::{
        ledger::{bitcoin, Ethereum},
        rfc003::{DeriveIdentities, SecretHash},
        HashFunction, SwapId, SwapProtocol,
    },
    timestamp::Timestamp,
};
use anyhow;
use libp2p_comit::frame::OutboundRequest;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// High-level message that represents a Swap request to another party
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request<AL, BL, AA, BA, AI, BI> {
    pub swap_id: SwapId,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub hash_function: HashFunction,
    pub alpha_ledger_refund_identity: AI,
    pub beta_ledger_redeem_identity: BI,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl
    TryFrom<
        Request<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Bitcoin, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinMainnet.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Ether(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<PublicKey, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinTestnet.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Ether(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<PublicKey, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinRegtest.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Ether(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<PublicKey, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinMainnet.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Erc20(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<PublicKey, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinTestnet.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Erc20(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<PublicKey, identity::Ethereum> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::BitcoinRegtest.to_header()?;
        let beta_ledger = LedgerKind::Ethereum(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::Bitcoin(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Erc20(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Mainnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Mainnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinMainnet.to_header()?;
        let alpha_asset = AssetKind::Ether(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Testnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Testnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinTestnet.to_header()?;
        let alpha_asset = AssetKind::Ether(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("beta_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Regtest,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Regtest,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinRegtest.to_header()?;
        let alpha_asset = AssetKind::Ether(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Mainnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Mainnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinMainnet.to_header()?;
        let alpha_asset = AssetKind::Erc20(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Testnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Testnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinTestnet.to_header()?;
        let alpha_asset = AssetKind::Erc20(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl
    TryFrom<
        Request<
            Ethereum,
            bitcoin::Regtest,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for OutboundRequest
{
    type Error = anyhow::Error;

    fn try_from(
        request: Request<
            Ethereum,
            bitcoin::Regtest,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<Self> {
        let request_body: RequestBody<identity::Ethereum, PublicKey> =
            RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::Ethereum(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::BitcoinRegtest.to_header()?;
        let alpha_asset = AssetKind::Erc20(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::Bitcoin(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

/// High-level message that represents accepting a Swap request
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Accept<AI, BI> {
    pub swap_id: SwapId,
    pub beta_ledger_refund_identity: BI,
    pub alpha_ledger_redeem_identity: AI,
}

/// High-level message that represents declining a Swap request
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Decline {
    pub swap_id: SwapId,
    pub reason: Option<SwapDeclineReason>,
}

/// Body of the rfc003 request message
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestBody<AI, BI> {
    pub alpha_ledger_refund_identity: AI,
    pub beta_ledger_redeem_identity: BI,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl<AL, BL, AA, BA, AI, BI> From<Request<AL, BL, AA, BA, AI, BI>> for RequestBody<AI, BI> {
    fn from(request: Request<AL, BL, AA, BA, AI, BI>) -> Self {
        RequestBody {
            alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
            alpha_expiry: request.alpha_expiry,
            beta_expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Decision {
    Accepted,
    Declined,
}

/// Body of the rfc003 accept message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AcceptResponseBody<AI, BI> {
    pub beta_ledger_refund_identity: BI,
    pub alpha_ledger_redeem_identity: AI,
}

/// Body of the rfc003 decline message
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclineResponseBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SwapDeclineReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwapDeclineReason {
    UnsatisfactoryRate,
    UnsupportedProtocol,
    UnsupportedSwap,
    MissingMandatoryHeader,
    BadJsonField,
}

pub trait IntoAcceptMessage<AI, BI> {
    fn into_accept_message(
        self,
        id: SwapId,
        secret_source: &dyn DeriveIdentities,
    ) -> Accept<AI, BI>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_empty_decline_body() {
        let decline_response_body = DeclineResponseBody { reason: None };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn serialize_decline_body_unsupported_protocol() {
        let decline_response_body = DeclineResponseBody {
            reason: Some(SwapDeclineReason::UnsupportedProtocol),
        };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{"reason":"unsupported-protocol"}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn serialize_decline_body_unsupported_swap() {
        let decline_response_body = DeclineResponseBody {
            reason: Some(SwapDeclineReason::UnsupportedSwap),
        };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{"reason":"unsupported-swap"}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn serialize_decline_body_missing_mandatory_header() {
        let decline_response_body = DeclineResponseBody {
            reason: Some(SwapDeclineReason::MissingMandatoryHeader),
        };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{"reason":"missing-mandatory-header"}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn serialize_decline_body_bad_json_field() {
        let decline_response_body = DeclineResponseBody {
            reason: Some(SwapDeclineReason::BadJsonField),
        };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{"reason":"bad-json-field"}"#;

        assert_eq!(response, expected_response);
    }
}
