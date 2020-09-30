use crate::{ethereum::ether, Result};

#[derive(Debug, Clone)]
pub struct Client;

impl Client {
    pub fn new(_url: url::Url) -> Self {
        todo!()
    }

    pub async fn gas_price(&self) -> Result<ether::Amount> {
        todo!()
    }
}
