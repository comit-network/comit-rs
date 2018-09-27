use bitcoin_support;

#[derive(Debug, Serialize)]
pub struct BitcoinQuery {
    pub to_address: Option<bitcoin_support::Address>,
}
