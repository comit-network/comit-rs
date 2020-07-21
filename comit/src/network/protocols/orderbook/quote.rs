use anyhow::bail;
use serde::{Deserialize, Serialize};

const PRECISION: usize = 6;

/// An indirect quote i.e., the amount of the quote currency required
/// to buy one unit of the base currency. We use indirect quotes
/// because we currently only support BTC/DAI and it is more useful to
/// quote the amount of DAI required to buy a single Bitcoin.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Quote {
    /// The value stored as an integer `floor(initial * precision)`.
    raw: usize,
    /// Number of decimal places included.
    precision: usize,
}

impl Quote {
    pub fn new(precision: usize) -> Self {
        Quote { raw: 0, precision }
    }

    pub fn from_float(f: f32) -> anyhow::Result<Quote> {
        Quote::default().with_value(f)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    pub fn with_value(&mut self, f: f32) -> anyhow::Result<Quote> {
        if f.is_sign_negative() {
            bail!("quotes must be positive");
        }

        // Indirect quote, expect bitcoin to be above US$1000 and below US$10000000
        debug_assert!(f > 1000.0); // debug only, ok to use float comparison.
        debug_assert!(f < 1000000.0);

        let x = 10.0_f32;
        let multiplier = x.powi(self.precision as i32);

        let r = f * multiplier as f32;
        let r = r.floor();
        let raw = r as usize;

        if raw == 0 {
            bail!(
                "qoute is too small, should be within {} decimal places",
                self.precision,
            );
        }

        Ok(Quote {
            raw,
            precision: self.precision,
        })
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    pub fn to_float(&self) -> f32 {
        let x = 10.0_f32;
        let multiplier = x.powi(self.precision as i32);
        self.raw as f32 / multiplier as f32
    }
}

impl Default for Quote {
    fn default() -> Self {
        Quote {
            raw: 0,
            precision: PRECISION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn simple_valid_functionality() {
        let f = 9_000.123;
        let precision = 2;
        let q = Quote::new(precision)
            .with_value(f)
            .expect("failed to construct quote");

        let want = 9_000.12;
        let got = q.to_float();

        assert_that(&got).is_equal_to(&want);
    }
}
