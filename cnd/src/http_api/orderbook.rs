use crate::{
    asset::{
        Erc20Quantity, {self},
    },
    ethereum,
    network::BtcDaiOrderAddresses,
    Facade, Role,
};
use comit::{order, BtcDaiOrderForm, Position};
use serde::Deserialize;
use time::NumericalDuration;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn post_make_btc_dai_order(
    body: MakeBtcDaiOrderBody,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    let (form, addresses) = body.into_form_and_addresses();
    let _ = facade.swarm.publish_order(form, addresses).await;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct MakeBtcDaiOrderBody {
    position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    quantity: asset::Bitcoin,
    price: Erc20Quantity,
    swap: SwapParams,
}

#[derive(Debug, Deserialize)]
pub struct SwapParams {
    #[serde(default = "default_role")]
    role: Role,
    bitcoin_address: bitcoin::Address,
    ethereum_address: ethereum::Address,
}

fn default_role() -> Role {
    Role::Alice
}

impl MakeBtcDaiOrderBody {
    fn into_form_and_addresses(self) -> (BtcDaiOrderForm, BtcDaiOrderAddresses) {
        let position = self.position;
        let swap_protocol =
            order::SwapProtocol::new(self.swap.role, position, 24.hours(), 12.hours()); // TODO: fill in good expiries here

        let form = BtcDaiOrderForm {
            position,
            quantity: self.quantity,
            price: self.price,
            swap_protocol,
        };
        let addresses = BtcDaiOrderAddresses {
            bitcoin: self.swap.bitcoin_address,
            ethereum: self.swap.ethereum_address,
        };

        (form, addresses)
    }
}
