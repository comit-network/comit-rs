use std::fmt;
use u256_ext::RemoveTrailingZeros;
use web3::types::Address;
use U256;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Quantity {
    token: Erc20Token,
    amount: U256,
}

impl Erc20Quantity {
    pub fn new(symbol: String, decimals: u16, address: Address, wei: U256) -> Self {
        Erc20Quantity {
            token: Erc20Token {
                symbol,
                decimals,
                address,
            },
            amount: wei,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.token.symbol
    }

    pub fn address(&self) -> Address {
        self.token.address.clone()
    }

    pub fn decimals(&self) -> u16 {
        self.token.decimals
    }

    pub fn amount(&self) -> U256 {
        self.amount
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Erc20Token {
    pub symbol: String,
    pub decimals: u16,
    pub address: Address,
}

impl fmt::Display for Erc20Quantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let removed_trailing_zeros = self
            .amount
            .remove_trailing_zeros(self.token.decimals.into());
        write!(f, "{} {}", removed_trailing_zeros, self.token.symbol)
    }
}
