use bitcoin_support;

#[derive(Clone, Debug, Serialize)]
pub struct BitcoinQuery {
    pub to_address: Option<bitcoin_support::Address>,
}
