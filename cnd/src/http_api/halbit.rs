use crate::{
    asset, identity, ledger,
    lnd::{AddHoldInvoice, Chain, SendPayment, SettleInvoice},
    RelativeTime, Secret, SecretHash,
};

pub use crate::halbit::*;

/// Data for the halbit protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Copy, Debug)]
pub struct Halbit {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub amount: asset::Bitcoin,
    pub identity: identity::Lightning,
    pub network: ledger::Bitcoin,
    pub cltv_expiry: u32,
}

impl From<Halbit> for CreatedSwap {
    fn from(p: Halbit) -> Self {
        CreatedSwap {
            asset: p.amount,
            identity: p.identity,
            network: p.network,
            cltv_expiry: p.cltv_expiry,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Finalized {
    pub asset: asset::Bitcoin,
    pub network: ledger::Bitcoin,
    pub refund_identity: identity::Lightning,
    pub redeem_identity: identity::Lightning,
    pub cltv_expiry: RelativeTime,
    pub state: State,
}

impl Finalized {
    pub fn build_init_action(&self, secret_hash: SecretHash) -> AddHoldInvoice {
        let amount = self.asset;
        let expiry = INVOICE_EXPIRY_SECS;
        let cltv_expiry = self.cltv_expiry;
        let chain = Chain::Bitcoin;
        let network = self.network;
        let self_public_key = self.redeem_identity;

        AddHoldInvoice {
            amount,
            secret_hash,
            expiry,
            cltv_expiry,
            chain,
            network,
            self_public_key,
        }
    }

    pub fn build_fund_action(&self, secret_hash: SecretHash) -> SendPayment {
        let to_public_key = self.redeem_identity;
        let amount = self.asset;
        let final_cltv_delta = self.cltv_expiry;
        let chain = Chain::Bitcoin;
        let network = self.network;
        let self_public_key = self.refund_identity;

        SendPayment {
            to_public_key,
            amount,
            secret_hash,
            final_cltv_delta,
            chain,
            network,
            self_public_key,
        }
    }

    pub fn build_redeem_action(&self, secret: Secret) -> SettleInvoice {
        let chain = Chain::Bitcoin;
        let network = self.network;
        let self_public_key = self.redeem_identity;

        SettleInvoice {
            secret,
            chain,
            network,
            self_public_key,
        }
    }
}
