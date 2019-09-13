use crate::http_api::action::ListRequiredFields;
use comit::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::Accept,
        messages::{AcceptResponseBody, IntoAcceptResponseBody},
        Ledger, SecretSource,
    },
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRedeem<L: Ledger> {
    pub alpha_ledger_redeem_identity: L::Identity,
}

impl ListRequiredFields for Accept<Ethereum, Bitcoin> {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![siren::Field {
            name: "alpha_ledger_redeem_identity".to_owned(),
            class: vec!["ethereum".to_owned(), "address".to_owned()],
            _type: Some("text".to_owned()),
            value: None,
            title: Some("Alpha ledger redeem identity".to_owned()),
        }]
    }
}

impl IntoAcceptResponseBody<Ethereum, Bitcoin> for OnlyRedeem<Ethereum> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Ethereum, Bitcoin> {
        AcceptResponseBody {
            alpha_ledger_redeem_identity: self.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity: secret_source.secp256k1_refund().into(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRefund<L: Ledger> {
    pub beta_ledger_refund_identity: L::Identity,
}

impl ListRequiredFields for Accept<Bitcoin, Ethereum> {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![siren::Field {
            name: "beta_ledger_refund_identity".to_owned(),
            class: vec!["ethereum".to_owned(), "address".to_owned()],
            _type: Some("text".to_owned()),
            value: None,
            title: Some("Beta ledger refund identity".to_owned()),
        }]
    }
}

impl IntoAcceptResponseBody<Bitcoin, Ethereum> for OnlyRefund<Ethereum> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Bitcoin, Ethereum> {
        AcceptResponseBody {
            beta_ledger_refund_identity: self.beta_ledger_refund_identity,
            alpha_ledger_redeem_identity: secret_source.secp256k1_redeem().into(),
        }
    }
}
