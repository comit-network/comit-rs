use crate::{
    http_api::action::ListRequiredFields,
    identity,
    swap_protocols::{
        ledger,
        rfc003::{
            actions::Accept,
            messages::{self, IntoAcceptMessage},
            Rfc003DeriveIdentities, SwapId,
        },
    },
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRedeem<I> {
    pub alpha_ledger_redeem_identity: I,
}

impl ListRequiredFields for Accept<ledger::Ethereum, ledger::Bitcoin> {
    fn list_required_fields() -> Vec<siren::Field> {
        ethereum_bitcoin_accept_required_fields()
    }
}

fn ethereum_bitcoin_accept_required_fields() -> Vec<siren::Field> {
    vec![siren::Field {
        name: "alpha_ledger_redeem_identity".to_owned(),
        class: vec!["ethereum".to_owned(), "address".to_owned()],
        _type: Some("text".to_owned()),
        value: None,
        title: Some("Alpha ledger redeem identity".to_owned()),
    }]
}

impl IntoAcceptMessage<identity::Ethereum, identity::Bitcoin> for OnlyRedeem<identity::Ethereum> {
    fn into_accept_message(
        self,
        id: SwapId,
        secret_source: &dyn Rfc003DeriveIdentities,
    ) -> messages::Accept<identity::Ethereum, identity::Bitcoin> {
        let beta_ledger_refund_identity = identity::Bitcoin::from_secret_key(
            &*crate::SECP,
            &secret_source.rfc003_derive_refund_identity(),
        );
        messages::Accept {
            swap_id: id,
            alpha_ledger_redeem_identity: self.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRefund<I> {
    pub beta_ledger_refund_identity: I,
}

impl ListRequiredFields for Accept<ledger::Bitcoin, ledger::Ethereum> {
    fn list_required_fields() -> Vec<siren::Field> {
        bitcoin_ethereum_accept_required_fields()
    }
}

fn bitcoin_ethereum_accept_required_fields() -> Vec<siren::Field> {
    vec![siren::Field {
        name: "beta_ledger_refund_identity".to_owned(),
        class: vec!["ethereum".to_owned(), "address".to_owned()],
        _type: Some("text".to_owned()),
        value: None,
        title: Some("Beta ledger refund identity".to_owned()),
    }]
}

impl IntoAcceptMessage<identity::Bitcoin, identity::Ethereum> for OnlyRefund<identity::Ethereum> {
    fn into_accept_message(
        self,
        id: SwapId,
        secret_source: &dyn Rfc003DeriveIdentities,
    ) -> messages::Accept<identity::Bitcoin, identity::Ethereum> {
        let alpha_ledger_redeem_identity = identity::Bitcoin::from_secret_key(
            &*crate::SECP,
            &secret_source.rfc003_derive_redeem_identity(),
        );
        messages::Accept {
            swap_id: id,
            beta_ledger_refund_identity: self.beta_ledger_refund_identity,
            alpha_ledger_redeem_identity,
        }
    }
}
