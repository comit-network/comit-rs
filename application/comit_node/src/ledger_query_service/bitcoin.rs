use bitcoin_support;

#[derive(Serialize)]
pub struct BitcoinQuery {
    pub to_address: Option<bitcoin_support::Address>,
}
