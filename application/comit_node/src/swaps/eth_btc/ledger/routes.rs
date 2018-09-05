use common_types::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use swaps::{alice_events::ContractDeployed, common::TradeId, errors::Error};

#[derive(Deserialize)]
pub struct ContractDeployedRequestBody {
    pub contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-contract-deployed",
    format = "application/json",
    data = "<contract_deployed_request_body>"
)]
pub fn post_contract_deployed(
    trade_id: TradeId,
    contract_deployed_request_body: Json<ContractDeployedRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<(), BadRequest<String>> {
    handle_post_contract_deployed(
        event_store.inner(),
        trade_id,
        contract_deployed_request_body.into_inner().contract_address,
    )?;

    Ok(())
}

fn handle_post_contract_deployed(
    event_store: &InMemoryEventStore<TradeId>,
    uid: TradeId,
    address: ethereum_support::Address,
) -> Result<(), Error> {
    let deployed: ContractDeployed<Ethereum, Bitcoin> = ContractDeployed::new(uid, address);
    event_store.add_event(uid, deployed)?;

    Ok(())
}
