use crate::network::Taker;
use std::collections::HashSet;

// TODO: Find a better name
#[derive(Default, Debug)]
pub struct OngoingTakers(HashSet<Taker>);

impl OngoingTakers {
    pub fn insert(&mut self, taker: Taker) -> anyhow::Result<()> {
        if self.0.contains(&taker) {
            anyhow::bail!("taker {} is already part of ongoing takers", taker.0)
        }

        self.0.insert(taker);
        Ok(())
    }

    pub fn cannot_trade_with_taker(&self, taker: &Taker) -> bool {
        self.0.contains(taker)
    }

    fn remove(&mut self, taker: &Taker) {
        self.0.remove(taker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_new_taker_is_ok() {
        let mut state = OngoingTakers::default();
        let taker = Taker::default();

        let insertion = state.insert(taker);

        assert!(insertion.is_ok());
    }

    #[test]
    fn insert_taker_a_second_time_fails() {
        let mut state = OngoingTakers::default();
        let taker = Taker::default();

        let insertion_1 = state.insert(taker);
        let insertion_2 = state.insert(taker);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_err());
    }

    #[test]
    fn insert_two_different_takers_is_ok() {
        let mut state = OngoingTakers::default();
        let taker_1 = Taker::new(1);
        let taker_2 = Taker::new(2);

        let insertion_1 = state.insert(taker_1);
        let insertion_2 = state.insert(taker_2);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_ok());
    }

    #[test]
    fn insert_remove_and_insert_same_taker_is_ok() {
        let mut state = OngoingTakers::default();
        let taker = Taker::default();

        let insertion_1 = state.insert(taker);
        state.remove(&taker);
        let insertion_2 = state.insert(taker);

        assert!(insertion_1.is_ok());
        assert!(insertion_2.is_ok());
    }
}
