use crate::{
    http_api::action::ListRequiredFields,
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            actions::Accept,
            messages::{self, IntoAcceptMessage},
            Ledger, SecretSource,
        },
        SwapId,
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

impl IntoAcceptMessage<Ethereum, Bitcoin> for OnlyRedeem<Ethereum> {
    fn into_accept_message(
        self,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> messages::Accept<Ethereum, Bitcoin> {
        let beta_ledger_refund_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_refund(),
        );
        messages::Accept {
            id,
            alpha_ledger_redeem_identity: self.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
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

impl IntoAcceptMessage<Bitcoin, Ethereum> for OnlyRefund<Ethereum> {
    fn into_accept_message(
        self,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> messages::Accept<Bitcoin, Ethereum> {
        let alpha_ledger_redeem_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_redeem(),
        );
        messages::Accept {
            id,
            beta_ledger_refund_identity: self.beta_ledger_refund_identity,
            alpha_ledger_redeem_identity,
        }
    }
}
