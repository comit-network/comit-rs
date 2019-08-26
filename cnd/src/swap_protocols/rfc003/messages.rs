use crate::swap_protocols::{
    asset::Asset,
    rfc003::{Ledger, SecretHash, SecretSource},
    HashFunction, Timestamp,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
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

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Decision {
    Accepted,
    Declined,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AcceptResponseBody<AL: Ledger, BL: Ledger> {
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_ledger_redeem_identity: AL::Identity,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeclineResponseBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SwapDeclineReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SwapDeclineReason {
    UnsatisfactoryRate,
    UnsupportedProtocol,
    UnsupportedSwap,
    MissingMandatoryHeader,
    UnexpectedJsonField,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestBody<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

pub trait ToRequest<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    fn to_request(&self, secret_source: &dyn SecretSource) -> Request<AL, BL, AA, BA>;
}

pub trait IntoAcceptResponseBody<AL: Ledger, BL: Ledger> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<AL, BL>;
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
    fn serialize_decline_body_unexpected_json_field() {
        let decline_response_body = DeclineResponseBody {
            reason: Some(SwapDeclineReason::UnexpectedJsonField),
        };

        let response = serde_json::to_string(&decline_response_body).unwrap();
        let expected_response = r#"{"reason":"unexpected-json-field"}"#;

        assert_eq!(response, expected_response);
    }
}
