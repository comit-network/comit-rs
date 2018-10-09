use bitcoin_support::BitcoinQuantity;
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use swaps::{
    alice_events::ContractDeployed as AliceContractDeployed, common::TradeId, errors::Error,
};

#[derive(Deserialize, Debug)]
pub struct AliceContractDeployedRequestBody {
    pub contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-contract-deployed",
    format = "application/json",
    data = "<contract_deployed_request_body>"
)]
pub fn post_contract_deployed(
    trade_id: TradeId,
    contract_deployed_request_body: Json<AliceContractDeployedRequestBody>,
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
) -> Result<(), BadRequest<String>> {
    let event_store = event_store.inner();
    handle_post_contract_deployed(
        event_store,
        trade_id,
        contract_deployed_request_body.into_inner().contract_address,
    )?;

    Ok(())
}

fn handle_post_contract_deployed(
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    uid: TradeId,
    address: ethereum_support::Address,
) -> Result<(), Error> {
    let deployed: AliceContractDeployed<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity> =
        AliceContractDeployed::new(uid, address);
    event_store.add_event(uid, deployed)?;

    Ok(())
}
