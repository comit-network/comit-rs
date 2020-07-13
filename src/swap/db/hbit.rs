use crate::{
    swap,
    swap::{
        db::{Database, Load, Save},
        hbit,
    },
    SwapId,
};
use ::bitcoin::secp256k1;
use anyhow::{anyhow, Context};
use comit::{identity, Secret, SecretHash, Timestamp};
use serde::{Deserialize, Serialize};

// TODO: control the serialisation
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct HbitFunded {
    pub asset: Amount,
    pub location: ::bitcoin::OutPoint,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Amount(u64);

impl From<Amount> for comit::asset::Bitcoin {
    fn from(amount: Amount) -> Self {
        comit::asset::Bitcoin::from_sat(amount.0)
    }
}

impl From<comit::asset::Bitcoin> for Amount {
    fn from(amount: comit::asset::Bitcoin) -> Self {
        Amount(amount.as_sat())
    }
}

impl From<HbitFunded> for hbit::Funded {
    fn from(funded: HbitFunded) -> Self {
        hbit::Funded {
            asset: funded.asset.into(),
            location: funded.location,
        }
    }
}

impl From<hbit::Funded> for HbitFunded {
    fn from(funded: hbit::Funded) -> Self {
        HbitFunded {
            asset: funded.asset.into(),
            location: funded.location,
        }
    }
}

impl Save<hbit::Funded> for Database {
    fn save(&self, event: hbit::Funded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_funded {
            Some(_) => Err(anyhow!("Hbit Funded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_funded = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

impl Load<hbit::Funded> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Funded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_funded.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HbitRedeemed {
    pub transaction: comit::transaction::Bitcoin,
    pub secret: Secret,
}

impl From<HbitRedeemed> for hbit::Redeemed {
    fn from(event: HbitRedeemed) -> Self {
        hbit::Redeemed {
            transaction: event.transaction,
            secret: event.secret,
        }
    }
}

impl From<hbit::Redeemed> for HbitRedeemed {
    fn from(event: hbit::Redeemed) -> Self {
        HbitRedeemed {
            transaction: event.transaction,
            secret: event.secret,
        }
    }
}

impl Save<hbit::Redeemed> for Database {
    fn save(&self, event: hbit::Redeemed, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_redeemed {
            Some(_) => Err(anyhow!("Hbit Redeemed event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_redeemed = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

impl Load<hbit::Redeemed> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Redeemed>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_redeemed.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HbitRefunded {
    pub transaction: comit::transaction::Bitcoin,
}

impl From<HbitRefunded> for hbit::Refunded {
    fn from(event: HbitRefunded) -> Self {
        hbit::Refunded {
            transaction: event.transaction,
        }
    }
}

impl From<hbit::Refunded> for HbitRefunded {
    fn from(event: hbit::Refunded) -> Self {
        HbitRefunded {
            transaction: event.transaction,
        }
    }
}

impl Save<hbit::Refunded> for Database {
    fn save(&self, event: hbit::Refunded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_refunded {
            Some(_) => Err(anyhow!("Hbit Refunded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_refunded = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

impl Load<hbit::Refunded> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Refunded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_refunded.map(Into::into))
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Params {
    pub network: ::bitcoin::Network,
    pub asset: Amount,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
    pub transient_sk: secp256k1::SecretKey,
}

impl From<Params> for swap::hbit::Params {
    fn from(_: Params) -> Self {
        todo!()
    }
}

#[cfg(test)]
impl Default for Params {
    fn default() -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap::db::{Database, Swap};

    fn bitcoin_transaction() -> ::bitcoin::Transaction {
        ::bitcoin::Transaction {
            version: 1,
            lock_time: 0,
            input: vec![::bitcoin::TxIn {
                previous_output: Default::default(),
                script_sig: Default::default(),
                sequence: 0,
                witness: vec![],
            }],
            output: vec![::bitcoin::TxOut {
                value: 0,
                script_pubkey: Default::default(),
            }],
        }
    }

    #[test]
    fn save_and_load_hbit_funded() {
        let db = Database::new_test().unwrap();
        let asset = comit::asset::Bitcoin::from_sat(123456);
        let location = comit::htlc_location::Bitcoin::default();
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db._insert(&swap_id, &swap).unwrap();

        let funded = hbit::Funded { asset, location };
        db.save(funded, swap_id).unwrap();

        let stored_funded: hbit::Funded = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_funded.asset, asset);
        assert_eq!(stored_funded.location, location);
    }

    #[test]
    fn save_and_load_hbit_redeemed() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let secret = Secret::from_vec(b"are those thirty-two bytes? Hum.").unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db._insert(&swap_id, &swap).unwrap();

        let event = hbit::Redeemed {
            transaction: transaction.clone(),
            secret,
        };
        db.save(event, swap_id).unwrap();

        let stored_event: hbit::Redeemed = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.secret, secret);
    }

    #[test]
    fn save_and_load_hbit_refunded() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db._insert(&swap_id, &swap).unwrap();

        let event = hbit::Refunded {
            transaction: transaction.clone(),
        };
        db.save(event, swap_id).unwrap();

        let stored_event: hbit::Refunded = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
    }
}
