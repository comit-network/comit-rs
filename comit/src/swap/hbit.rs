use crate::{asset, hbit::Redeemed, htlc_location};
use anyhow::Result;
use bitcoin::secp256k1::SecretKey;
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Params {
    pub shared: crate::hbit::Params,
    pub transient_sk: SecretKey,
    pub final_address: bitcoin::Address,
}

impl Params {
    pub fn new(
        shared: crate::hbit::Params,
        transient_sk: SecretKey,
        final_address: bitcoin::Address,
    ) -> Self {
        Self {
            shared,
            transient_sk,
            final_address,
        }
    }
}

#[derive(Debug, Clone, Copy, Error)]
#[error("hbit HTLC was incorrectly funded, expected {expected} but got {got}")]
pub struct IncorrectlyFunded {
    pub expected: asset::Bitcoin,
    pub got: asset::Bitcoin,
}

#[async_trait::async_trait]
pub trait WatchForFunded {
    async fn watch_for_funded(
        &self,
        params: &Params,
        start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait WatchForRedeemed {
    async fn watch_for_redeemed(
        &self,
        params: &Params,
        fund_event: Funded,
        start_of_swap: OffsetDateTime,
    ) -> Redeemed;
}

#[derive(Debug, Clone, Copy)]
pub struct Funded {
    pub asset: asset::Bitcoin,
    pub location: htlc_location::Bitcoin,
}

#[cfg(feature = "quickcheck")]
mod arbitrary {
    use crate::{asset, identity, ledger, swap::hbit::Params, SecretHash, Timestamp};
    use ::bitcoin::secp256k1::SecretKey;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Params {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Params {
                shared: crate::hbit::Params {
                    network: bitcoin_network(g),
                    asset: bitcoin_asset(g),
                    redeem_identity: bitcoin_identity(g),
                    refund_identity: bitcoin_identity(g),
                    expiry: Timestamp::arbitrary(g),
                    secret_hash: SecretHash::arbitrary(g),
                },
                transient_sk: secret_key(g),
                final_address: bitcoin_address(g),
            }
        }
    }

    fn secret_key<G: Gen>(g: &mut G) -> SecretKey {
        let mut bytes = [0u8; 32];
        for byte in &mut bytes {
            *byte = u8::arbitrary(g);
        }
        SecretKey::from_slice(&bytes).unwrap()
    }

    fn bitcoin_network<G: Gen>(g: &mut G) -> ledger::Bitcoin {
        match u8::arbitrary(g) % 3 {
            0 => ledger::Bitcoin::Mainnet,
            1 => ledger::Bitcoin::Testnet,
            2 => ledger::Bitcoin::Regtest,
            _ => unreachable!(),
        }
    }

    fn bitcoin_asset<G: Gen>(g: &mut G) -> asset::Bitcoin {
        asset::Bitcoin::from_sat(u64::arbitrary(g))
    }

    fn bitcoin_identity<G: Gen>(g: &mut G) -> identity::Bitcoin {
        identity::Bitcoin::from_secret_key(
            &bitcoin::secp256k1::Secp256k1::signing_only(),
            &secret_key(g),
        )
    }

    fn bitcoin_address<G: Gen>(g: &mut G) -> bitcoin::Address {
        bitcoin::Address::p2wpkh(&bitcoin_identity(g).into(), bitcoin_network(g).into()).unwrap()
    }
}
