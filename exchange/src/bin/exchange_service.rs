extern crate exchange;
extern crate rocket;

use exchange::rocket_factory::create_rocket_instance;
use exchange::types::Offers;
use exchange::types::TreasuryApiUrl;
use std::env::var;

fn main() {
    let treasury_api_url = TreasuryApiUrl(var("TREASURY_SERVICE_URL").unwrap());
    let offers = Offers::new();
    create_rocket_instance(treasury_api_url, offers).launch();
}
