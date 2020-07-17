use crate::network::Taker;
use anyhow::Context;
use std::{collections::HashSet, iter::FromIterator};

/// Set of takers which have an ongoing trade with this instance of
/// Nectar. A trade is ongoing from the moment we confirm the
/// corresponding order till the moment the swap protocol ends.
#[derive(Debug)]
pub struct ActiveTakers {
    db: sled::Db,
    #[cfg(test)]
    tmp_dir: tempdir::TempDir,
}

impl ActiveTakers {
    #[cfg(not(test))]
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("The path is not utf-8 valid: {:?}", path))?;
        let db = sled::open(path).context(format!("Could not open the DB at {}", path))?;

        if !db.contains_key("takers")? {
            let takers = Vec::<Taker>::new();
            let takers = serde_json::to_vec(&takers)?;
            let _ = db.insert("takers", takers)?;
        }

        Ok(Self { db })
    }

    #[cfg(test)]
    pub fn new_test() -> anyhow::Result<Self> {
        let tmp_dir = tempdir::TempDir::new("nectar_test").unwrap();
        let db = sled::open(tmp_dir.path()).context(format!(
            "Could not open the DB at {}",
            tmp_dir.path().display()
        ))?;

        let takers = Vec::<Taker>::new();
        let takers = serde_json::to_vec(&takers)?;
        let _ = db.insert("takers", takers)?;

        Ok(Self { db, tmp_dir })
    }

    pub fn insert(&mut self, taker: Taker) -> anyhow::Result<()> {
        self.modify_with(|takers: &mut HashSet<Taker>| takers.insert(taker.clone()))
    }

    pub fn contains(&self, taker: &Taker) -> anyhow::Result<bool> {
        let takers = self.takers()?;

        Ok(takers.contains(&taker))
    }

    pub fn remove(&mut self, taker: &Taker) -> anyhow::Result<()> {
        self.modify_with(|takers: &mut HashSet<Taker>| takers.remove(taker))
    }

    fn modify_with(
        &mut self,
        operation_fn: impl Fn(&mut HashSet<Taker>) -> bool,
    ) -> anyhow::Result<()> {
        let mut takers = self.takers()?;

        operation_fn(&mut takers);

        let updated_takers = Vec::<Taker>::from_iter(takers);
        let updated_takers = serde_json::to_vec(&updated_takers)?;

        self.db.insert("takers", updated_takers)?;

        Ok(())
    }

    fn takers(&self) -> anyhow::Result<HashSet<Taker>> {
        let takers = self
            .db
            .get("takers")?
            .ok_or_else(|| anyhow::anyhow!("no key \"takers\" in db"))?;
        let takers: Vec<Taker> = serde_json::from_slice(&takers)?;
        let takers = HashSet::<Taker>::from_iter(takers);

        Ok(takers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taker_no_longer_has_ongoing_trade_after_removal() {
        let mut state = ActiveTakers::new_test().unwrap();
        let taker = Taker::default();

        let _ = state.insert(taker.clone()).unwrap();
        let res = state.contains(&taker);

        assert!(matches!(res, Ok(true)));

        let _ = state.remove(&taker).unwrap();
        let res = state.contains(&taker);

        assert!(matches!(res, Ok(false)));
    }
}
