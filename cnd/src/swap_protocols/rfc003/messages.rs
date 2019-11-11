use crate::swap_protocols::{
    asset::Asset,
    rfc003::{Ledger, SecretHash, SecretSource},
    HashFunction, SwapId, Timestamp,
};
use serde::{Deserialize, Serialize};

/// High-level message that represents a Swap request to another party
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub id: SwapId,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub hash_function: HashFunction,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

/// High-level message that represents accepting a Swap request
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    pub id: SwapId,
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_ledger_redeem_identity: AL::Identity,
}

/// High-level message that represents declining a Swap request
///
/// This does _not_ represent the actual network message, that is why it also
/// does not implement Serialize.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decline {
    pub id: SwapId,
    pub reason: Option<SwapDeclineReason>,
}

/// Body of the rfc003 request message
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestBody<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Decision {
    Accepted,
    Declined,
}

/// Body of the rfc003 accept message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AcceptResponseBody<AL: Ledger, BL: Ledger> {
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_ledger_redeem_identity: AL::Identity,
}

/// Body of the rfc003 decline message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeclineResponseBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SwapDeclineReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwapDeclineReason {
    UnsatisfactoryRate,
    UnsupportedProtocol,
    UnsupportedSwap,
    MissingMandatoryHeader,
    BadJsonField,
}

pub trait ToRequest<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    fn to_request(&self, id: SwapId, secret_source: &dyn SecretSource) -> Request<AL, BL, AA, BA>;
}

pub trait IntoAcceptMessage<AL: Ledger, BL: Ledger> {
    fn into_accept_message(self, id: SwapId, secret_source: &dyn SecretSource) -> Accept<AL, BL>;
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
